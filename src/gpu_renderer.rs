use crate::camera::Camera;
use crate::rasterizer::{RasterizedMap, MAP_HEIGHT, MAP_WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use crate::types::{MapMode, Province};
use glow::HasContext;
use std::sync::Arc;

pub struct GpuRenderer {
    gl: Arc<glow::Context>,
    program: glow::Program,
    quad_vao: glow::VertexArray,
    id_texture: glow::Texture,
    palette_texture: glow::Texture,

    palette_buffer: Vec<u8>,
}

impl GpuRenderer {
    pub fn new(gl: Arc<glow::Context>, raster_map: &RasterizedMap) -> Self {
        unsafe {
            let vs_src = r#"#version 330 core
                layout (location = 0) in vec2 a_pos;
                out vec2 v_texcoord;
                void main() {
                    v_texcoord = (a_pos + 1.0) * 0.5;
                    gl_Position = vec4(a_pos, 0.0, 1.0);
                }
            "#;

            let fs_src = r#"#version 330 core
                in vec2 v_texcoord;
                out vec4 fragColor;

                uniform sampler2D u_id_texture;
                uniform sampler2D u_palette_texture;
                uniform vec4 u_viewport; // x, y, width, height in physical pixels
                uniform vec2 u_pan;
                uniform float u_zoom;
                uniform float u_dpr;
                uniform int u_hovered_id;
                uniform int u_selected_id;
                uniform int u_show_borders;
                uniform vec4 u_ocean_color;
                uniform vec4 u_border_color;
                uniform vec2 u_id_tex_size;
                uniform vec2 u_world_size;

                int decodeId(vec4 texel) {
                    int r = int(texel.r * 255.0 + 0.5);
                    int g = int(texel.g * 255.0 + 0.5);
                    int b = int(texel.b * 255.0 + 0.5);
                    return (r << 16) | (g << 8) | b;
                }

                void main() {
                    // Screen coordinate relative to viewport top-left in logical points
                    float pixel_x = gl_FragCoord.x - u_viewport.x;
                    float pixel_y = (u_viewport.y + u_viewport.w) - gl_FragCoord.y;

                    vec2 logical_screen = vec2(pixel_x, pixel_y) / u_dpr;
                    vec2 world_coord = (logical_screen - u_pan) / u_zoom;
                    vec2 uv = world_coord / u_world_size;

                    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
                        fragColor = u_ocean_color;
                        return;
                    }

                    vec2 tex_uv = uv;
                    vec4 id_texel = texture(u_id_texture, tex_uv);
                    int id = decodeId(id_texel);

                    if (id == 0) {
                        fragColor = u_ocean_color;
                        return;
                    }

                    // Sample base faction/province color from 2D palette
                    ivec2 palette_coord = ivec2(id % 256, id / 256);
                    vec4 color = texelFetch(u_palette_texture, palette_coord, 0);

                    // Dynamic Multi-Tier Border Detection
                    if (u_show_borders == 1) {
                        vec2 step_offset = 1.0 / (u_world_size * u_zoom);
                        vec2 offset = clamp(step_offset, 0.4 / u_id_tex_size, 2.0 / u_id_tex_size);

                        int id_l = decodeId(texture(u_id_texture, tex_uv + vec2(-offset.x, 0.0)));
                        int id_r = decodeId(texture(u_id_texture, tex_uv + vec2( offset.x, 0.0)));
                        int id_u = decodeId(texture(u_id_texture, tex_uv + vec2(0.0,  offset.y)));
                        int id_d = decodeId(texture(u_id_texture, tex_uv + vec2(0.0, -offset.y)));

                        vec4 col_l = texelFetch(u_palette_texture, ivec2(id_l % 256, id_l / 256), 0);
                        vec4 col_r = texelFetch(u_palette_texture, ivec2(id_r % 256, id_r / 256), 0);
                        vec4 col_u = texelFetch(u_palette_texture, ivec2(id_u % 256, id_u / 256), 0);
                        vec4 col_d = texelFetch(u_palette_texture, ivec2(id_d % 256, id_d / 256), 0);

                        bool is_coastline = (id_l == 0 || id_r == 0 || id_u == 0 || id_d == 0);
                        bool is_national_border = (id_l != 0 && col_l != color) ||
                                                  (id_r != 0 && col_r != color) ||
                                                  (id_u != 0 && col_u != color) ||
                                                  (id_d != 0 && col_d != color);
                        bool is_province_border = (id_l != id || id_r != id || id_u != id || id_d != id);

                        if (is_coastline) {
                            color = mix(color, vec4(0.0, 0.1, 0.12, 1.0), 0.75);
                        } else if (is_national_border) {
                            color = mix(color, vec4(0.05, 0.05, 0.05, 1.0), 0.7);
                        } else if (is_province_border && u_zoom > 1.8) {
                            float inner_alpha = clamp((u_zoom - 1.8) / 3.5, 0.0, 0.4);
                            color = mix(color, vec4(0.12, 0.12, 0.12, 1.0), inner_alpha);
                        }
                    }

                    // Selection Glow
                    if (id == u_selected_id) {
                        color = mix(color, vec4(0.0, 0.85, 1.0, 1.0), 0.55);
                    }

                    // Hover Highlight Glow
                    if (id == u_hovered_id && id != u_selected_id) {
                        color = mix(color, vec4(1.0, 0.88, 0.2, 1.0), 0.45);
                    }

                    fragColor = color;
                }
            "#;

            let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vs, vs_src);
            gl.compile_shader(vs);

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fs, fs_src);
            gl.compile_shader(fs);

            let program = gl.create_program().unwrap();
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);

            gl.delete_shader(vs);
            gl.delete_shader(fs);

            // Quad VAO
            let quad_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(quad_vao));

            let quad_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            let vertices: [f32; 12] = [
                -1.0, -1.0,
                 1.0, -1.0,
                -1.0,  1.0,
                -1.0,  1.0,
                 1.0, -1.0,
                 1.0,  1.0,
            ];
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

            // ID Texture (NEAREST filtering)
            let id_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(id_texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                raster_map.width as i32,
                raster_map.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&raster_map.rgba_texture_data)),
            );

            // 2D 256x128 Palette Texture
            let palette_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(palette_texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);

            let palette_buffer = vec![0u8; 256 * 128 * 4];

            Self {
                gl,
                program,
                quad_vao,
                id_texture,
                palette_texture,
                palette_buffer,
            }
        }
    }

    pub fn update_palette(
        &mut self,
        provinces: &[Province],
        map_mode: MapMode,
        background: [u8; 3],
        default_color: [u8; 3],
    ) {
        // ID 0 = Ocean
        self.palette_buffer[0] = background[0];
        self.palette_buffer[1] = background[1];
        self.palette_buffer[2] = background[2];
        self.palette_buffer[3] = 255;

        for p in provinces {
            let id = p.index + 1;
            let offset = id * 4;
            if offset + 3 >= self.palette_buffer.len() {
                continue;
            }

            let color = match map_mode {
                MapMode::Political => {
                    if p.group_key.is_empty() {
                        default_color
                    } else {
                        p.color
                    }
                }
                MapMode::Wastelands => {
                    if p.is_wasteland {
                        [35, 35, 35]
                    } else {
                        [180, 190, 200]
                    }
                }
                MapMode::Independent => {
                    if p.group_key.is_empty() {
                        [255, 140, 0] // Bright orange for unassigned / neutral
                    } else {
                        [75, 90, 105] // Muted slate for claimed
                    }
                }
                MapMode::Plain => default_color,
            };

            self.palette_buffer[offset] = color[0];
            self.palette_buffer[offset + 1] = color[1];
            self.palette_buffer[offset + 2] = color[2];
            self.palette_buffer[offset + 3] = 255;
        }

        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                256,
                128,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&self.palette_buffer)),
            );
        }
    }

    pub fn render(
        &self,
        camera: &Camera,
        viewport_rect: [f32; 4], // x, y, width, height in physical pixels
        dpr: f32,
        hovered_idx: Option<usize>,
        selected_idx: Option<usize>,
        show_borders: bool,
    ) {
        unsafe {
            let gl = &self.gl;
            gl.viewport(
                viewport_rect[0] as i32,
                viewport_rect[1] as i32,
                viewport_rect[2] as i32,
                viewport_rect[3] as i32,
            );
            gl.use_program(Some(self.program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.id_texture));
            if let Some(loc) = gl.get_uniform_location(self.program, "u_id_texture") {
                gl.uniform_1_i32(Some(&loc), 0);
            }

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            if let Some(loc) = gl.get_uniform_location(self.program, "u_palette_texture") {
                gl.uniform_1_i32(Some(&loc), 1);
            }

            if let Some(loc) = gl.get_uniform_location(self.program, "u_viewport") {
                gl.uniform_4_f32(
                    Some(&loc),
                    viewport_rect[0],
                    viewport_rect[1],
                    viewport_rect[2],
                    viewport_rect[3],
                );
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_pan") {
                gl.uniform_2_f32(Some(&loc), camera.pan_x, camera.pan_y);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_zoom") {
                gl.uniform_1_f32(Some(&loc), camera.zoom);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_dpr") {
                gl.uniform_1_f32(Some(&loc), dpr);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_hovered_id") {
                gl.uniform_1_i32(Some(&loc), hovered_idx.map(|i| (i + 1) as i32).unwrap_or(-1));
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_selected_id") {
                gl.uniform_1_i32(Some(&loc), selected_idx.map(|i| (i + 1) as i32).unwrap_or(-1));
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_show_borders") {
                gl.uniform_1_i32(Some(&loc), if show_borders { 1 } else { 0 });
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_ocean_color") {
                gl.uniform_4_f32(Some(&loc), 0.004, 0.247, 0.247, 1.0); // #013f3f
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_border_color") {
                gl.uniform_4_f32(Some(&loc), 0.0, 0.0, 0.0, 0.7);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_id_tex_size") {
                gl.uniform_2_f32(Some(&loc), MAP_WIDTH as f32, MAP_HEIGHT as f32);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_world_size") {
                gl.uniform_2_f32(Some(&loc), WORLD_WIDTH, WORLD_HEIGHT);
            }

            gl.bind_vertex_array(Some(self.quad_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}
