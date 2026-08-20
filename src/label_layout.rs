use crate::types::{MapGroup, Province};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GlyphPlacement {
    pub char_val: char,
    pub world_pos: [f32; 2],
    pub angle: f32, // Radians
}

#[derive(Clone, Debug)]
pub struct NationLabel {
    pub group_key: String,
    pub name: String,
    pub glyphs: Vec<GlyphPlacement>,
    pub center: [f32; 2],
    pub province_count: usize,
    pub base_font_size: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

pub fn generate_nation_labels(
    groups: &HashMap<String, MapGroup>,
    provinces: &[Province],
) -> Vec<NationLabel> {
    let mut labels = Vec::new();

    // Map province ID to index for fast lookup
    let mut id_to_province = HashMap::with_capacity(provinces.len());
    for p in provinces {
        id_to_province.insert(&p.id, p);
    }

    for (group_key, group) in groups {
        let label_name = group.label.trim();
        if label_name.is_empty()
            || label_name.eq_ignore_ascii_case("wastelands")
            || label_name.eq_ignore_ascii_case("unassigned")
            || label_name.contains("BLESS THE RAINS")
            || label_name.contains("NE VALYAY")
            || label_name.contains("CHTO SIBIR")
            || label_name.contains("LAND DOWN UNDA")
        {
            continue;
        }

        let mut pts = Vec::new();
        for path_id in &group.paths {
            if let Some(p) = id_to_province.get(path_id) {
                pts.push(p.centroid);
            }
        }

        if pts.is_empty() {
            continue;
        }

        if let Some(nation_label) = build_curved_label(group_key, label_name, &pts) {
            labels.push(nation_label);
        }
    }

    // Sort so smaller/regional nations render above large empires
    labels.sort_by(|a, b| b.province_count.cmp(&a.province_count));
    labels
}

pub fn build_single_label(
    group_key: &str,
    group: &MapGroup,
    provinces: &[Province],
) -> Option<NationLabel> {
    let label_name = group.label.trim();
    if label_name.is_empty()
        || label_name.eq_ignore_ascii_case("wastelands")
        || label_name.eq_ignore_ascii_case("unassigned")
    {
        return None;
    }

    let mut id_to_province = HashMap::with_capacity(provinces.len());
    for p in provinces {
        id_to_province.insert(&p.id, p);
    }

    let mut pts = Vec::new();
    for path_id in &group.paths {
        if let Some(p) = id_to_province.get(path_id) {
            pts.push(p.centroid);
        }
    }

    if pts.is_empty() {
        return None;
    }

    build_curved_label(group_key, label_name, &pts)
}

fn build_curved_label(group_key: &str, name: &str, pts: &[[f32; 2]]) -> Option<NationLabel> {
    let n = pts.len();
    if n == 0 {
        return None;
    }

    // Format text: Convert to uppercase for classic Roman imperial cartography look
    let uppercase_name: String = name.to_uppercase();
    let chars: Vec<char> = uppercase_name.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // 1. Center of Mass
    let cx: f32 = pts.iter().map(|p| p[0]).sum::<f32>() / n as f32;
    let cy: f32 = pts.iter().map(|p| p[1]).sum::<f32>() / n as f32;

    if n == 1 || chars.len() == 1 {
        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            glyphs: vec![GlyphPlacement {
                char_val: chars[0],
                world_pos: [cx, cy],
                angle: 0.0,
            }],
            center: [cx, cy],
            province_count: n,
            base_font_size: 11.0,
            min_zoom: 6.0,
            max_zoom: 120.0,
        });
    }

    // 2. 2D Covariance Matrix for Principal Component Analysis (PCA)
    let mut cov_xx = 0.0f32;
    let mut cov_yy = 0.0f32;
    let mut cov_xy = 0.0f32;

    for p in pts {
        let dx = p[0] - cx;
        let dy = p[1] - cy;
        cov_xx += dx * dx;
        cov_yy += dy * dy;
        cov_xy += dx * dy;
    }
    cov_xx /= n as f32;
    cov_yy /= n as f32;
    cov_xy /= n as f32;

    // 3. Principal Eigenvector (Major Axis Orientation)
    let trace = cov_xx + cov_yy;
    let det = cov_xx * cov_yy - cov_xy * cov_xy;
    let lambda1 = trace * 0.5 + ((trace * 0.5).powi(2) - det).max(0.0).sqrt();

    let (mut vx, mut vy) = if cov_xy.abs() > 1e-5 {
        let x = lambda1 - cov_yy;
        let y = cov_xy;
        let norm = (x * x + y * y).sqrt();
        if norm > 1e-6 {
            (x / norm, y / norm)
        } else {
            (1.0, 0.0)
        }
    } else if cov_xx >= cov_yy {
        (1.0, 0.0)
    } else {
        (0.0, 1.0)
    };

    // Ensure principal vector points generally rightward for left-to-right reading
    if vx < 0.0 || (vx.abs() < 1e-4 && vy < 0.0) {
        vx = -vx;
        vy = -vy;
    }

    // 4. Project points onto principal axis to determine geographic spread
    let mut min_u = f32::INFINITY;
    let mut max_u = f32::NEG_INFINITY;
    let mut projections: Vec<(f32, [f32; 2])> = Vec::with_capacity(n);

    for p in pts {
        let u = (p[0] - cx) * vx + (p[1] - cy) * vy;
        if u < min_u {
            min_u = u;
        }
        if u > max_u {
            max_u = u;
        }
        projections.push((u, *p));
    }

    let spread_length = (max_u - min_u).max(8.0);

    // 5. Fit 3-Point Medial Bézier Curve along Principal Axis
    // Slices: Left (10%..40%), Middle (40%..60%), Right (60%..90%)
    let u_span = max_u - min_u;
    let slice1_max = min_u + u_span * 0.38;
    let slice2_min = min_u + u_span * 0.38;
    let slice2_max = min_u + u_span * 0.62;
    let slice3_min = min_u + u_span * 0.62;

    let mut p0_acc = [0.0f32; 2];
    let mut p0_count = 0;
    let mut p1_acc = [0.0f32; 2];
    let mut p1_count = 0;
    let mut p2_acc = [0.0f32; 2];
    let mut p2_count = 0;

    for (u, pt) in &projections {
        if *u <= slice1_max {
            p0_acc[0] += pt[0];
            p0_acc[1] += pt[1];
            p0_count += 1;
        } else if *u >= slice2_min && *u <= slice2_max {
            p1_acc[0] += pt[0];
            p1_acc[1] += pt[1];
            p1_count += 1;
        } else if *u >= slice3_min {
            p2_acc[0] += pt[0];
            p2_acc[1] += pt[1];
            p2_count += 1;
        }
    }

    let p0 = if p0_count > 0 {
        [p0_acc[0] / p0_count as f32, p0_acc[1] / p0_count as f32]
    } else {
        [cx + min_u * 0.7 * vx, cy + min_u * 0.7 * vy]
    };

    let p1 = if p1_count > 0 {
        [p1_acc[0] / p1_count as f32, p1_acc[1] / p1_count as f32]
    } else {
        [cx, cy]
    };

    let p2 = if p2_count > 0 {
        [p2_acc[0] / p2_count as f32, p2_acc[1] / p2_count as f32]
    } else {
        [cx + max_u * 0.7 * vx, cy + max_u * 0.7 * vy]
    };

    // Quadratic Bézier curve evaluation helper
    let eval_bezier = |t: f32| -> ([f32; 2], [f32; 2]) {
        let it = 1.0 - t;
        let it2 = it * it;
        let t2 = t * t;
        let it_t_2 = 2.0 * it * t;

        // Position B(t)
        let bx = it2 * p0[0] + it_t_2 * p1[0] + t2 * p2[0];
        let by = it2 * p0[1] + it_t_2 * p1[1] + t2 * p2[1];

        // Derivative B'(t) = 2*(1-t)*(P1-P0) + 2*t*(P2-P1)
        let dx = 2.0 * it * (p1[0] - p0[0]) + 2.0 * t * (p2[0] - p1[0]);
        let dy = 2.0 * it * (p1[1] - p0[1]) + 2.0 * t * (p2[1] - p1[1]);

        ([bx, by], [dx, dy])
    };

    // 6. Character Distribution along Bézier Spine
    let num_chars = chars.len();
    let mut glyphs = Vec::with_capacity(num_chars);

    // Paradox Sprawl Scaling: Spacing is proportional to territory width and name length
    let sprawl_ratio = (spread_length / (num_chars as f32 * 14.0)).clamp(0.4, 1.2);
    let half_span = (0.35 * sprawl_ratio).clamp(0.2, 0.45);
    let t_start = 0.5 - half_span;
    let t_end = 0.5 + half_span;
    let t_range = t_end - t_start;

    for i in 0..num_chars {
        let t = if num_chars == 1 {
            0.5
        } else {
            t_start + (i as f32 / (num_chars - 1) as f32) * t_range
        };

        let (pos, tangent) = eval_bezier(t);
        let mut angle = tangent[1].atan2(tangent[0]);

        // Normalize angle to [-PI/2, PI/2] so letters are always read right-side up
        if angle > std::f32::consts::FRAC_PI_2 {
            angle -= std::f32::consts::PI;
        } else if angle < -std::f32::consts::FRAC_PI_2 {
            angle += std::f32::consts::PI;
        }

        glyphs.push(GlyphPlacement {
            char_val: chars[i],
            world_pos: pos,
            angle,
        });
    }

    // 7. Determine Zoom Level of Detail (LOD) thresholds based on nation size
    let (base_font_size, min_zoom, max_zoom) = if n >= 200 {
        // Major Great Empire (e.g. Ming, France, Inca, Japan, Ottomans)
        (16.0, 0.4, 40.0)
    } else if n >= 80 {
        // Large Kingdom (e.g. Poland, Castile, England, Hungary)
        (14.0, 0.8, 60.0)
    } else if n >= 20 {
        // Medium Kingdom / Regional Power
        (12.0, 1.5, 80.0)
    } else if n >= 5 {
        // Duchy / Minor Realm
        (11.0, 3.0, 100.0)
    } else {
        // Tiny County / City-State
        (10.0, 6.0, 120.0)
    };

    Some(NationLabel {
        group_key: group_key.to_string(),
        name: name.to_string(),
        glyphs,
        center: [cx, cy],
        province_count: n,
        base_font_size,
        min_zoom,
        max_zoom,
    })
}
