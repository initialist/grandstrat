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
    relief_texture: glow::Texture,
    ocean_texture: glow::Texture,
    terrain_texture: glow::Texture,

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
                uniform sampler2D u_relief_texture;
                uniform sampler2D u_ocean_texture;
                uniform sampler2D u_terrain_texture;
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
                uniform float u_time;
                uniform int u_map_mode; // 0 = Political, 1 = Terrain, 2 = Wastelands, 3 = Independent, 4 = Plain
                uniform float u_relief_strength;

                int decodeId(vec4 texel) {
                    int r = int(texel.r * 255.0 + 0.5);
                    int g = int(texel.g * 255.0 + 0.5);
                    int b = int(texel.b * 255.0 + 0.5);
                    return (r << 16) | (g << 8) | b;
                }

                float hash21(vec2 p) {
                    p = fract(p * vec2(123.34, 456.21));
                    p += dot(p, p + 45.32);
                    return fract(p.x * p.y);
                }

                void main() {
                    // Screen coordinate relative to viewport top-left in logical points
                    float pixel_x = gl_FragCoord.x - u_viewport.x;
                    float pixel_y = (u_viewport.y + u_viewport.w) - gl_FragCoord.y;

                    vec2 logical_screen = vec2(pixel_x, pixel_y) / u_dpr;
                    vec2 world_coord = (logical_screen - u_pan) / u_zoom;
                    vec2 uv = world_coord / u_world_size;

                    // Micro tactile paper grain
                    float paper_grain = hash21(world_coord * 14.0) * 0.04 - 0.02;

                    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
                        fragColor = u_ocean_color;
                        return;
                    }

                    vec2 tex_uv = uv;
                    vec4 id_texel = texture(u_id_texture, tex_uv);
                    int id = decodeId(id_texel);

                    // Screen-Space Consistent Multi-Tap Edge & Coastal Detection (Refined & Subtle)
                    // Clamped to at most 2.0 texels to prevent thick borders when zoomed out
                    vec2 offset = clamp(vec2(0.65) / (u_world_size * u_zoom), 1.0 / u_id_tex_size, 2.2 / u_id_tex_size);

                    int id_l = decodeId(texture(u_id_texture, tex_uv + vec2(-offset.x, 0.0)));
                    int id_r = decodeId(texture(u_id_texture, tex_uv + vec2( offset.x, 0.0)));
                    int id_u = decodeId(texture(u_id_texture, tex_uv + vec2(0.0,  offset.y)));
                    int id_d = decodeId(texture(u_id_texture, tex_uv + vec2(0.0, -offset.y)));

                    int land_taps = (id_l > 0 ? 1 : 0) + (id_r > 0 ? 1 : 0) + (id_u > 0 ? 1 : 0) + (id_d > 0 ? 1 : 0);

                    // Exact MapChart SVG to Equirectangular Earth Coordinate Alignment
                    vec2 earth_uv;
                    earth_uv.x = fract(uv.x + 0.0308);
                    earth_uv.y = clamp(uv.y * 0.8592 + 0.0478, 0.0, 1.0);

                    // ----------------------------------------------------------------
                    // Ocean Shading: Pure Seabed Bathymetry (Zero Land Contamination)
                    // ----------------------------------------------------------------
                    if (id == 0) {
                        float shelf_proximity = float(land_taps) / 4.0;
                        
                        // Sample pure ocean bathymetry (trenches, abyssal plains, ridges)
                        vec3 ocean_bathymetry = texture(u_ocean_texture, earth_uv).rgb;
                        
                        vec3 deep_navy = vec3(0.04, 0.22, 0.32); // Radiant oceanic blue
                        vec3 shelf_turquoise = vec3(0.12, 0.50, 0.58); // Coastal shelf turquoise

                        vec3 ocean_base = mix(deep_navy, ocean_bathymetry * 1.30, 0.72);
                        ocean_base = mix(ocean_base, shelf_turquoise, shelf_proximity * 0.55);

                        // Subtle scale-independent water shimmer (zero moire)
                        float wave = sin(logical_screen.x * 0.04 + u_time * 1.2) * cos(logical_screen.y * 0.04 + u_time * 0.9) * 0.012;
                        ocean_base += vec3(wave);

                        // Delicate coastal wave foam along shores
                        if (land_taps > 0) {
                            float foam = sin(u_time * 2.2 - float(land_taps) * 0.9) * 0.5 + 0.5;
                            ocean_base = mix(ocean_base, vec3(0.88, 0.96, 0.98), shelf_proximity * foam * 0.35);
                        }

                        ocean_base += paper_grain * 0.25;
                        fragColor = vec4(clamp(ocean_base, 0.0, 1.0), 1.0);
                        return;
                    }

                    // ----------------------------------------------------------------
                    // True 3D Cartographic Relief & Wasteland-Aligned Topography
                    // - Inhabited Lands: Softer, elegant, readable terrain (max 26% shadow)
                    // - Wasteland Mountain Spines: Rugged, sharp 3D crags & physical shadows
                    // - Biomes: Lush forest canopy stippling, desert dunes, and river channels
                    // ----------------------------------------------------------------
                    // Base Land Color Selection
                    ivec2 palette_coord = ivec2(id % 256, id / 256);
                    vec4 raw_color = texelFetch(u_palette_texture, palette_coord, 0);
                    vec3 land_rgb = raw_color.rgb;

                    // Wasteland check (color #525252 or wastelands map mode)
                    bool is_wasteland = (raw_color.r > 0.30 && raw_color.r < 0.35 && raw_color.g > 0.30 && raw_color.g < 0.35 && raw_color.b > 0.30 && raw_color.b < 0.35) || (u_map_mode == 2);

                    // 1. Perspective Parallax Displacement
                    vec2 view_center = vec2(0.5, 0.5);
                    vec2 view_dir = uv - view_center;
                    float base_h = texture(u_relief_texture, earth_uv).r;
                    float elev_offset = max(base_h - 0.706, 0.0);
                    float parallax_scale = is_wasteland ? 0.0018 : 0.0008;
                    vec2 parallax_uv = earth_uv + view_dir * elev_offset * parallax_scale * clamp(u_zoom * 0.10, 0.4, 1.4);

                    // 2. Horizon Cast Shadows (Prominent on wastelands, subtle on inhabited)
                    vec2 sun_ray = vec2(-0.00042, -0.00042);
                    float shadow_occlusion = 0.0;
                    for (int i = 1; i <= 3; i++) {
                        float fi = float(i);
                        float h_sample = texture(u_relief_texture, parallax_uv + sun_ray * fi).r;
                        float diff = h_sample - (base_h + fi * 0.020);
                        if (diff > 0.0) {
                            shadow_occlusion += diff * (1.0 / fi);
                        }
                    }
                    shadow_occlusion = clamp(shadow_occlusion * 2.2, 0.0, 1.0);

                    // 3. Slope Normal Gradient & Laplacian Cavity AO
                    vec2 r_step = vec2(0.00035, 0.00035);
                    float r_c = texture(u_relief_texture, parallax_uv).r;
                    float r_l = texture(u_relief_texture, parallax_uv - vec2(r_step.x, 0.0)).r;
                    float r_r = texture(u_relief_texture, parallax_uv + vec2(r_step.x, 0.0)).r;
                    float r_u = texture(u_relief_texture, parallax_uv - vec2(0.0, r_step.y)).r;
                    float r_d = texture(u_relief_texture, parallax_uv + vec2(0.0, r_step.y)).r;

                    // 3D Slope Normal Gradient (315° NW Sun Direction)
                    float slope_x = (r_r - r_l) * 7.5;
                    float slope_y = (r_d - r_u) * 7.5;
                    vec2 sun_dir = vec2(-0.7071, -0.7071);
                    float slope_sun = dot(vec2(slope_x, slope_y), sun_dir);

                    // Laplacian Cavity Curvature (Detects drainage basins & rolling plains)
                    float cavity = (r_l + r_r + r_u + r_d - 4.0 * r_c) * 10.0;

                    // Sample Satellite / Biome Landcover Texture
                    vec4 terrain_tex = texture(u_terrain_texture, parallax_uv);

                    if (u_map_mode == 1) {
                        // Real Hypsometric & Satellite Landcover Biomes Mode
                        land_rgb = terrain_tex.rgb * 1.35;
                    }

                    // Shading distinction: Harsh on Wasteland vs Soft on Inhabited
                    vec3 shaded_land;
                    if (is_wasteland) {
                        // Rugged alpine crags and deep rock shadows on wasteland mountain ranges
                        float w_shade = clamp(1.0 + min(slope_sun + cavity * 0.40, 0.0) * 0.70 - shadow_occlusion * 0.45, 0.38, 1.0);
                        float w_sun = max(slope_sun, 0.0) * 0.22;
                        shaded_land = land_rgb * w_shade + vec3(w_sun);
                    } else {
                        // Soft, balanced, readable hillshade on inhabited lands (max 26% darkening)
                        float in_shade = clamp(1.0 + min(slope_sun + cavity * 0.25, 0.0) * 0.32, 0.74, 1.0);
                        float in_sun = max(slope_sun, 0.0) * 0.10;
                        shaded_land = land_rgb * in_shade + vec3(in_sun);
                    }

                    // 4. Biome Texturing: Forest & Woodland Canopies
                    float forest_index = clamp((terrain_tex.g * 1.8 - terrain_tex.r * 0.9 - terrain_tex.b * 0.9) * 2.8, 0.0, 1.0);
                    if (forest_index > 0.08 && !is_wasteland) {
                        float tree_stipple = (sin(uv.x * 6200.0) * cos(uv.y * 6200.0) + sin(uv.x * 12400.0) * 0.5) * 0.026;
                        vec3 forest_tint = vec3(0.93, 0.98, 0.91);
                        shaded_land = mix(shaded_land, shaded_land * forest_tint + vec3(tree_stipple), forest_index * 0.45);
                    }

                    // 5. Biome Texturing: Desert Dunes & Arid Lowlands
                    float desert_index = clamp((terrain_tex.r * 1.3 + terrain_tex.g * 1.0 - terrain_tex.b * 1.8 - 0.4) * 2.5, 0.0, 1.0);
                    if (desert_index > 0.12 && !is_wasteland) {
                        float dune_ripple = sin((uv.x + uv.y) * 4500.0) * 0.015;
                        shaded_land += vec3(dune_ripple * desert_index);
                    }

                    // 6. Major Freshwater River Drainage Channels
                    float river_water = clamp((terrain_tex.b * 1.6 - terrain_tex.r * 0.8 - terrain_tex.g * 0.6) * 2.5, 0.0, 1.0);
                    if (river_water > 0.28 && !is_wasteland) {
                        vec3 river_blue = vec3(0.08, 0.40, 0.55);
                        shaded_land = mix(shaded_land, river_blue, (river_water - 0.28) * 0.70);
                    }

                    // Multi-Tier Border Detection (Subtle, Refined & Elegant)
                    if (u_show_borders == 1) {
                        vec4 col_l = texelFetch(u_palette_texture, ivec2(id_l % 256, id_l / 256), 0);
                        vec4 col_r = texelFetch(u_palette_texture, ivec2(id_r % 256, id_r / 256), 0);
                        vec4 col_u = texelFetch(u_palette_texture, ivec2(id_u % 256, id_u / 256), 0);
                        vec4 col_d = texelFetch(u_palette_texture, ivec2(id_d % 256, id_d / 256), 0);

                        bool is_coastline = (id_l == 0 || id_r == 0 || id_u == 0 || id_d == 0);
                        bool is_national_border = (id_l != 0 && col_l.rgb != raw_color.rgb) ||
                                                  (id_r != 0 && col_r.rgb != raw_color.rgb) ||
                                                  (id_u != 0 && col_u.rgb != raw_color.rgb) ||
                                                  (id_d != 0 && col_d.rgb != raw_color.rgb);
                        bool is_province_border = (id_l != id || id_r != id || id_u != id || id_d != id);

                        if (is_coastline) {
                            shaded_land = mix(shaded_land, vec3(0.02, 0.08, 0.12), 0.78);
                        } else if (is_national_border) {
                            // National / Group Borders: Clean, refined dark stroke (never thick or overwhelming)
                            shaded_land = mix(shaded_land, vec3(0.06, 0.06, 0.06), 0.72);
                        } else if (is_province_border) {
                            // Province borders: Very subtle hairline incision, visible only when zooming in close
                            float prov_fade = smoothstep(2.5, 6.5, u_zoom);
                            float prov_alpha = prov_fade * 0.28;
                            shaded_land = mix(shaded_land, vec3(0.15, 0.15, 0.15), prov_alpha);
                        }
                    }

                    // Tactile Parchment Paper Grain
                    shaded_land += vec3(paper_grain * 0.30);

                    vec4 color = vec4(shaded_land, 1.0);

                    // Selection Glow
                    if (id == u_selected_id) {
                        color = mix(color, vec4(0.0, 0.88, 1.0, 1.0), 0.55);
                    }

                    // Hover Highlight Glow
                    if (id == u_hovered_id && id != u_selected_id) {
                        color = mix(color, vec4(1.0, 0.88, 0.20, 1.0), 0.45);
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

            // 4K Pure Earth Shaded Relief Texture
            let relief_img = image::load_from_memory(include_bytes!("../assets/textures/earth_relief.jpg"))
                .expect("Failed to load earth_relief.jpg")
                .to_rgba8();
            let (r_w, r_h) = relief_img.dimensions();

            let relief_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(relief_texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                r_w as i32,
                r_h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(relief_img.as_raw())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);

            // 4K Pure Ocean Bottom Bathymetry Texture
            let ocean_img = image::load_from_memory(include_bytes!("../assets/textures/earth_ocean.jpg"))
                .expect("Failed to load earth_ocean.jpg")
                .to_rgba8();
            let (o_w, o_h) = ocean_img.dimensions();

            let ocean_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(ocean_texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                o_w as i32,
                o_h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(ocean_img.as_raw())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);

            // 4K Real Earth Satellite & Hypsometric Terrain Texture
            let terrain_img = image::load_from_memory(include_bytes!("../assets/textures/earth_terrain.jpg"))
                .expect("Failed to load earth_terrain.jpg")
                .to_rgba8();
            let (t_w, t_h) = terrain_img.dimensions();

            let terrain_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(terrain_texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                t_w as i32,
                t_h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(terrain_img.as_raw())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);

            Self {
                gl,
                program,
                quad_vao,
                id_texture,
                palette_texture,
                relief_texture,
                ocean_texture,
                terrain_texture,
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
                MapMode::Political | MapMode::Terrain => {
                    if p.group_key.is_empty() {
                        default_color
                    } else {
                        p.color
                    }
                }
                MapMode::Wastelands => {
                    if p.is_wasteland || p.group_label.to_lowercase().contains("wasteland") {
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
        map_mode: MapMode,
        relief_strength: f32,
        time: f32,
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

            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.relief_texture));
            if let Some(loc) = gl.get_uniform_location(self.program, "u_relief_texture") {
                gl.uniform_1_i32(Some(&loc), 2);
            }

            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.ocean_texture));
            if let Some(loc) = gl.get_uniform_location(self.program, "u_ocean_texture") {
                gl.uniform_1_i32(Some(&loc), 3);
            }

            gl.active_texture(glow::TEXTURE4);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.terrain_texture));
            if let Some(loc) = gl.get_uniform_location(self.program, "u_terrain_texture") {
                gl.uniform_1_i32(Some(&loc), 4);
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
                gl.uniform_4_f32(Some(&loc), 0.020, 0.145, 0.190, 1.0); // #052530
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
            if let Some(loc) = gl.get_uniform_location(self.program, "u_time") {
                gl.uniform_1_f32(Some(&loc), time);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_relief_strength") {
                gl.uniform_1_f32(Some(&loc), relief_strength);
            }

            let mode_int = match map_mode {
                MapMode::Political => 0,
                MapMode::Terrain => 1,
                MapMode::Wastelands => 2,
                MapMode::Independent => 3,
                MapMode::Plain => 4,
            };
            if let Some(loc) = gl.get_uniform_location(self.program, "u_map_mode") {
                gl.uniform_1_i32(Some(&loc), mode_int);
            }

            gl.bind_vertex_array(Some(self.quad_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}
