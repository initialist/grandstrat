use crate::types::{LabelAlgorithm, MapGroup, Province};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct NationLabelChar {
    pub ch: char,
    pub world_pos: [f32; 2],
    pub angle: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NationLabel {
    pub group_key: String,
    pub name: String,
    pub chars: Vec<NationLabelChar>,
    pub center: [f32; 2],
    pub world_span: f32,
    pub world_font_size: f32,
    pub province_count: usize,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

pub fn generate_nation_labels(
    groups: &HashMap<String, MapGroup>,
    provinces: &[Province],
    algorithm: LabelAlgorithm,
) -> Vec<NationLabel> {
    let mut labels = Vec::new();

    // Map province ID to index for fast lookup
    let mut id_to_province = HashMap::with_capacity(provinces.len());
    for p in provinces {
        id_to_province.insert(&p.id, p);
    }

    for (group_key, group) in groups {
        let label_name = group.label.trim();
        let upper = label_name.to_uppercase();
        if label_name.is_empty()
            || upper.contains("WASTELAND")
            || upper.contains("UNASSIGNED")
            || upper.contains("ASSORTED GROUPS")
            || upper.contains("BLESS THE RAINS")
            || upper.contains("NE VALYAY")
            || upper.contains("CHTO SIBIR")
            || upper.contains("LAND DOWN UNDA")
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

        // 1. Spatial Landmass Clustering: Separate main territory from scattered overseas islands
        let clusters = cluster_provinces(&pts, 25.0);
        if clusters.is_empty() {
            continue;
        }

        // Take primary (largest) contiguous landmass cluster
        let main_cluster = &clusters[0];
        if main_cluster.is_empty() {
            continue;
        }

        let nation_label = match algorithm {
            LabelAlgorithm::Standard => {
                build_standard_label(group_key, label_name, main_cluster, pts.len())
            }
            LabelAlgorithm::Curved => {
                build_curved_arc_label(group_key, label_name, main_cluster, pts.len())
            }
        };

        if let Some(label) = nation_label {
            labels.push(label);
        }
    }

    // Sort so smaller/regional nations render above large empires
    labels.sort_by(|a, b| b.province_count.cmp(&a.province_count));
    labels
}

/// Connected component clustering based on spatial proximity (distance threshold in world units)
fn cluster_provinces(pts: &[[f32; 2]], dist_threshold: f32) -> Vec<Vec<[f32; 2]>> {
    let n = pts.len();
    let thresh_sq = dist_threshold * dist_threshold;
    let mut clusters = Vec::new();
    let mut visited = vec![false; n];

    for i in 0..n {
        if visited[i] {
            continue;
        }

        let mut cluster = Vec::new();
        cluster.push(pts[i]);
        visited[i] = true;

        let mut queue = Vec::new();
        queue.push(pts[i]);

        while let Some(curr) = queue.pop() {
            for j in 0..n {
                if !visited[j] {
                    let dx = curr[0] - pts[j][0];
                    let dy = curr[1] - pts[j][1];
                    if dx * dx + dy * dy <= thresh_sq {
                        visited[j] = true;
                        cluster.push(pts[j]);
                        queue.push(pts[j]);
                    }
                }
            }
        }
        clusters.push(cluster);
    }

    // Sort clusters by province count descending (largest main territory first)
    clusters.sort_by(|a, b| b.len().cmp(&a.len()));
    clusters
}

/// 100% Preserved Standard Baseline Label Algorithm (Available for Reversion)
fn build_standard_label(
    group_key: &str,
    name: &str,
    pts: &[[f32; 2]],
    total_province_count: usize,
) -> Option<NationLabel> {
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

    // 1. True Center of Mass (Mean of Province Centroids)
    let cx: f32 = pts.iter().map(|p| p[0]).sum::<f32>() / n as f32;
    let cy: f32 = pts.iter().map(|p| p[1]).sum::<f32>() / n as f32;

    // Geometric Bounding Box
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for p in pts {
        if p[0] < min_x { min_x = p[0]; }
        if p[0] > max_x { max_x = p[0]; }
        if p[1] < min_y { min_y = p[1]; }
        if p[1] > max_y { max_y = p[1]; }
    }

    let box_w = (max_x - min_x).max(4.0);
    let box_h = (max_y - min_y).max(4.0);
    let thickness = box_w.min(box_h);
    let _geo_mean = (box_w * box_h).sqrt();

    // 2. Base Zoom Tier: All labels eligible for global visibility
    let (min_zoom, max_zoom) = (0.2, 180.0);

    if chars.len() == 1 {
        let label_chars = vec![NationLabelChar {
            ch: chars[0],
            world_pos: [cx, cy],
            angle: 0.0,
        }];
        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            chars: label_chars,
            center: [cx, cy],
            world_span: 6.0,
            world_font_size: 0.65,
            province_count: total_province_count,
            min_zoom,
            max_zoom,
        });
    }

    if n == 1 {
        // Single province nation: layout all letters horizontally centered at cx, cy with compact scaling
        let world_span = box_w.max(3.5);
        let target_span = world_span * 0.75;
        let num_chars = chars.len();
        let max_font_for_span = (target_span * 1.15) / (0.80 * (num_chars - 1).max(1) as f32 + 0.70);
        let world_font_size = (thickness * 0.20 + 0.22).min(max_font_for_span).clamp(0.20, 1.20);

        let ideal_step = (target_span - world_font_size * 0.70) / (num_chars - 1).max(1) as f32;
        let clamped_step = ideal_step.clamp(world_font_size * 0.85, world_font_size * 2.0);
        let total_world_w = (num_chars - 1) as f32 * clamped_step;
        let start_x = cx - total_world_w * 0.5;

        let mut label_chars = Vec::with_capacity(num_chars);
        for (i, &ch) in chars.iter().enumerate() {
            let x = start_x + i as f32 * clamped_step;
            let y = cy;
            label_chars.push(NationLabelChar {
                ch,
                world_pos: [x, y],
                angle: 0.0,
            });
        }

        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            chars: label_chars,
            center: [cx, cy],
            world_span,
            world_font_size,
            province_count: total_province_count,
            min_zoom,
            max_zoom,
        });
    }

    // 3. 2D Covariance Matrix for Principal Component Analysis (PCA)
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

    // 4. Eigenvalues & Aspect Ratio Analysis (Eduard Imhof Rule #1)
    let trace = cov_xx + cov_yy;
    let det = cov_xx * cov_yy - cov_xy * cov_xy;
    let disc = ((trace * 0.5).powi(2) - det).max(0.0).sqrt();
    let lambda1 = trace * 0.5 + disc;
    let lambda2 = (trace * 0.5 - disc).max(0.001);

    let aspect_ratio = (lambda1 / lambda2).sqrt();

    let num_chars = chars.len();
    let thickness = box_w.min(box_h);
    let geo_mean = (box_w * box_h).sqrt();

    // If the country is compact/squarish (aspect ratio < 1.7, like France, China, Byzantium, HRE)
    // or has very few provinces: standard horizontal placement centered at center-of-mass!
    if aspect_ratio < 1.7 || n <= 3 {
        let world_span = box_w.max(box_h * 0.85);
        let target_span = world_span * 0.62;

        let natural_font = (thickness * 0.16 + geo_mean * 0.05 + 0.35).clamp(0.4, 6.5);
        let max_font_for_span = (target_span * 1.15) / (0.80 * (num_chars - 1).max(1) as f32 + 0.70);
        let world_font_size = natural_font.min(max_font_for_span).clamp(0.25, 6.0);

        let ideal_step = (target_span - world_font_size * 0.70) / (num_chars - 1).max(1) as f32;
        let clamped_step = ideal_step.clamp(world_font_size * 0.85, world_font_size * 2.20);
        let total_world_w = (num_chars - 1) as f32 * clamped_step;
        let start_x = cx - total_world_w * 0.5;

        let mut label_chars = Vec::with_capacity(num_chars);
        for (i, &ch) in chars.iter().enumerate() {
            let x = start_x + i as f32 * clamped_step;
            let y = cy;
            label_chars.push(NationLabelChar {
                ch,
                world_pos: [x, y],
                angle: 0.0,
            });
        }

        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            chars: label_chars,
            center: [cx, cy],
            world_span,
            world_font_size,
            province_count: total_province_count,
            min_zoom,
            max_zoom,
        });
    }

    // 5. Elongated Feature: Fit Curved Bézier Spine along Principal Component Axis
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

    // Project points onto principal axis
    let mut min_u = f32::INFINITY;
    let mut max_u = f32::NEG_INFINITY;
    let mut projections: Vec<(f32, [f32; 2])> = Vec::with_capacity(n);

    for p in pts {
        let u = (p[0] - cx) * vx + (p[1] - cy) * vy;
        if u < min_u { min_u = u; }
        if u > max_u { max_u = u; }
        projections.push((u, *p));
    }
    projections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let p0_slice = &projections[(n as f32 * 0.08) as usize..(n as f32 * 0.28).min(n as f32) as usize];
    let p1_slice = &projections[(n as f32 * 0.40) as usize..(n as f32 * 0.60).min(n as f32) as usize];
    let p2_slice = &projections[(n as f32 * 0.72) as usize..(n as f32 * 0.92).min(n as f32) as usize];

    let mut p0 = if !p0_slice.is_empty() {
        let x = p0_slice.iter().map(|(_, p)| p[0]).sum::<f32>() / p0_slice.len() as f32;
        let y = p0_slice.iter().map(|(_, p)| p[1]).sum::<f32>() / p0_slice.len() as f32;
        [x, y]
    } else {
        [cx + min_u * 0.7 * vx, cy + min_u * 0.7 * vy]
    };

    let p1 = if !p1_slice.is_empty() {
        let x = p1_slice.iter().map(|(_, p)| p[0]).sum::<f32>() / p1_slice.len() as f32;
        let y = p1_slice.iter().map(|(_, p)| p[1]).sum::<f32>() / p1_slice.len() as f32;
        [x, y]
    } else {
        [cx, cy]
    };

    let mut p2 = if !p2_slice.is_empty() {
        let x = p2_slice.iter().map(|(_, p)| p[0]).sum::<f32>() / p2_slice.len() as f32;
        let y = p2_slice.iter().map(|(_, p)| p[1]).sum::<f32>() / p2_slice.len() as f32;
        [x, y]
    } else {
        [cx + max_u * 0.7 * vx, cy + max_u * 0.7 * vy]
    };

    // Smooth spine curvature: Clamp deviation from chord
    let chord_dx = p2[0] - p0[0];
    let chord_dy = p2[1] - p0[1];
    let chord_len = (chord_dx * chord_dx + chord_dy * chord_dy).sqrt().max(1.0);

    let mid_x = (p0[0] + p2[0]) * 0.5;
    let mid_y = (p0[1] + p2[1]) * 0.5;
    let dev_x = p1[0] - mid_x;
    let dev_y = p1[1] - mid_y;
    let max_dev = chord_len * 0.22;
    let cur_dev = (dev_x * dev_x + dev_y * dev_y).sqrt();

    let p1_clamped = if cur_dev > max_dev && cur_dev > 0.001 {
        let scale = max_dev / cur_dev;
        [mid_x + dev_x * scale, mid_y + dev_y * scale]
    } else {
        p1
    };

    // Strict Reading Direction Orientation:
    // 1. If primarily vertical (|dx| < 0.55 * |dy|): flow top-to-bottom (P0.y < P2.y)
    // 2. Otherwise (horizontal / diagonal): flow left-to-right (P0.x < P2.x)
    let c_dx = p2[0] - p0[0];
    let c_dy = p2[1] - p0[1];
    if c_dx.abs() < 0.55 * c_dy.abs() {
        if p2[1] < p0[1] {
            std::mem::swap(&mut p0, &mut p2);
        }
    } else if p2[0] < p0[0] {
        std::mem::swap(&mut p0, &mut p2);
    }

    // Compute accurate arc length of quadratic Bézier curve
    let eval_bezier = |t: f32| -> ([f32; 2], [f32; 2]) {
        let it = 1.0 - t;
        let it2 = it * it;
        let t2 = t * t;
        let it_t_2 = 2.0 * it * t;
        let bx = it2 * p0[0] + it_t_2 * p1_clamped[0] + t2 * p2[0];
        let by = it2 * p0[1] + it_t_2 * p1_clamped[1] + t2 * p2[1];
        let dx = 2.0 * it * (p1_clamped[0] - p0[0]) + 2.0 * t * (p2[0] - p1_clamped[0]);
        let dy = 2.0 * it * (p1_clamped[1] - p0[1]) + 2.0 * t * (p2[1] - p1_clamped[1]);
        ([bx, by], [dx, dy])
    };

    const SAMPLES: usize = 16;
    let mut arc_lens = [0.0f32; SAMPLES + 1];
    let mut prev_pt = eval_bezier(0.0).0;
    for k in 1..=SAMPLES {
        let t = k as f32 / SAMPLES as f32;
        let pt = eval_bezier(t).0;
        let d = ((pt[0] - prev_pt[0]).powi(2) + (pt[1] - prev_pt[1]).powi(2)).sqrt();
        arc_lens[k] = arc_lens[k - 1] + d;
        prev_pt = pt;
    }
    let total_arc_len = arc_lens[SAMPLES].max(1.0);

    let s_to_t = |s: f32| -> f32 {
        let s_clamped = s.clamp(0.0, total_arc_len);
        for k in 1..=SAMPLES {
            if arc_lens[k] >= s_clamped {
                let s_prev = arc_lens[k - 1];
                let s_next = arc_lens[k];
                let seg_t = if (s_next - s_prev).abs() > 1e-6 {
                    (s_clamped - s_prev) / (s_next - s_prev)
                } else {
                    0.0
                };
                let t_prev = (k - 1) as f32 / SAMPLES as f32;
                let t_next = k as f32 / SAMPLES as f32;
                return t_prev + seg_t * (t_next - t_prev);
            }
        }
        1.0
    };

    let is_vertical = c_dx.abs() < 0.55 * c_dy.abs();
    let num_chars = chars.len();

    let world_span = (max_u - min_u).max(box_w.max(box_h) * 0.75);
    let target_arc = total_arc_len * 0.65;

    let natural_font = (thickness * 0.16 + geo_mean * 0.05 + 0.35).clamp(0.4, 6.5);
    let max_font_for_span = (target_arc * 1.15) / (0.80 * (num_chars - 1).max(1) as f32 + 0.70);
    let world_font_size = natural_font.min(max_font_for_span).clamp(0.25, 6.0);

    let ideal_step = (target_arc - world_font_size * 0.70) / (num_chars - 1).max(1) as f32;
    let s_step = ideal_step.clamp(world_font_size * 0.85, world_font_size * 2.20);
    let total_world_arc = (num_chars - 1) as f32 * s_step;
    let s_start = ((total_arc_len - total_world_arc) * 0.5).max(0.0);

    let mut label_chars = Vec::with_capacity(num_chars);
    for (i, &ch) in chars.iter().enumerate() {
        let s = s_start + i as f32 * s_step;
        let t = s_to_t(s);
        let (pt, tangent) = eval_bezier(t);

        let angle = if is_vertical {
            0.0
        } else {
            let mut a = tangent[1].atan2(tangent[0]);
            if a > std::f32::consts::FRAC_PI_2 {
                a -= std::f32::consts::PI;
            } else if a < -std::f32::consts::FRAC_PI_2 {
                a += std::f32::consts::PI;
            }
            a.clamp(-0.48, 0.48)
        };

        label_chars.push(NationLabelChar {
            ch,
            world_pos: pt,
            angle,
        });
    }

    Some(NationLabel {
        group_key: group_key.to_string(),
        name: name.to_string(),
        chars: label_chars,
        center: [cx, cy],
        world_span,
        world_font_size,
        province_count: total_province_count,
        min_zoom,
        max_zoom,
    })
}

/// Advanced Geodesic Curved Arc Label Algorithm
/// Fits smooth polynomial/Bézier medial spines along physical landmass curvature
/// with per-letter tangent rotation and natural cartographic arching.
fn build_curved_arc_label(
    group_key: &str,
    name: &str,
    pts: &[[f32; 2]],
    total_province_count: usize,
) -> Option<NationLabel> {
    let n = pts.len();
    if n == 0 {
        return None;
    }

    let uppercase_name: String = name.to_uppercase();
    let chars: Vec<char> = uppercase_name.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // 1. Center of Mass
    let cx: f32 = pts.iter().map(|p| p[0]).sum::<f32>() / n as f32;
    let cy: f32 = pts.iter().map(|p| p[1]).sum::<f32>() / n as f32;

    // Bounding Box
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for p in pts {
        if p[0] < min_x { min_x = p[0]; }
        if p[0] > max_x { max_x = p[0]; }
        if p[1] < min_y { min_y = p[1]; }
        if p[1] > max_y { max_y = p[1]; }
    }

    let box_w = (max_x - min_x).max(4.0);
    let box_h = (max_y - min_y).max(4.0);
    let thickness = box_w.min(box_h);
    let geo_mean = (box_w * box_h).sqrt();
    let (min_zoom, max_zoom) = (0.2, 180.0);

    if chars.len() == 1 {
        let label_chars = vec![NationLabelChar {
            ch: chars[0],
            world_pos: [cx, cy],
            angle: 0.0,
        }];
        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            chars: label_chars,
            center: [cx, cy],
            world_span: 6.0,
            world_font_size: 0.65,
            province_count: total_province_count,
            min_zoom,
            max_zoom,
        });
    }

    if n == 1 {
        // Single province nation: layout all letters horizontally centered at cx, cy with compact scaling
        let world_span = box_w.max(3.5);
        let target_span = world_span * 0.75;
        let num_chars = chars.len();
        let max_font_for_span = (target_span * 1.15) / (0.80 * (num_chars - 1).max(1) as f32 + 0.70);
        let world_font_size = (thickness * 0.20 + 0.22).min(max_font_for_span).clamp(0.20, 1.20);

        let ideal_step = (target_span - world_font_size * 0.70) / (num_chars - 1).max(1) as f32;
        let clamped_step = ideal_step.clamp(world_font_size * 0.85, world_font_size * 2.0);
        let total_world_w = (num_chars - 1) as f32 * clamped_step;
        let start_x = cx - total_world_w * 0.5;

        let mut label_chars = Vec::with_capacity(num_chars);
        for (i, &ch) in chars.iter().enumerate() {
            let x = start_x + i as f32 * clamped_step;
            let y = cy;
            label_chars.push(NationLabelChar {
                ch,
                world_pos: [x, y],
                angle: 0.0,
            });
        }

        return Some(NationLabel {
            group_key: group_key.to_string(),
            name: name.to_string(),
            chars: label_chars,
            center: [cx, cy],
            world_span,
            world_font_size,
            province_count: total_province_count,
            min_zoom,
            max_zoom,
        });
    }

    // 2. 2D Covariance Matrix & Principal Components
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

    let trace = cov_xx + cov_yy;
    let det = cov_xx * cov_yy - cov_xy * cov_xy;
    let disc = ((trace * 0.5).powi(2) - det).max(0.0).sqrt();
    let lambda1 = trace * 0.5 + disc;
    let lambda2 = (trace * 0.5 - disc).max(0.001);
    let aspect_ratio = (lambda1 / lambda2).sqrt();

    // Primary elongation unit vector v1
    let (mut vx, mut vy) = if aspect_ratio < 1.65 {
        // Compact country (France, Poland, HRE, Byzantium, Castile, China):
        // Standard horizontal reading with gentle cartographic arch
        (1.0, 0.0)
    } else if cov_xy.abs() > 1e-5 {
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

    // Ensure forward reading direction (Left-to-Right for horizontal/diagonal, Top-to-Bottom for vertical)
    if vx.abs() >= 0.45 * vy.abs() {
        if vx < 0.0 {
            vx = -vx;
            vy = -vy;
        }
    } else if vy < 0.0 {
        vx = -vx;
        vy = -vy;
    }

    // Orthogonal unit vector v2 (lateral offset across spine)
    let (wx, wy) = (-vy, vx);

    // 3. Project points onto local (u, w) frame along spine
    let mut u_coords = Vec::with_capacity(n);
    let mut w_coords = Vec::with_capacity(n);
    for p in pts {
        let dx = p[0] - cx;
        let dy = p[1] - cy;
        let u = dx * vx + dy * vy;
        let w = dx * wx + dy * wy;
        u_coords.push(u);
        w_coords.push(w);
    }

    let min_u = u_coords.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_u = u_coords.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let u_span = (max_u - min_u).max(3.0);
    let u_half = u_span * 0.5;

    // 4. Fit smooth polynomial spine w(u) = a*u^2 + b*u + c
    let mut sum_u4 = 0.0f32;
    let mut sum_u3 = 0.0f32;
    let mut sum_u2 = 0.0f32;
    let mut sum_u1 = 0.0f32;
    let mut sum_w = 0.0f32;
    let mut sum_wu = 0.0f32;
    let mut sum_wu2 = 0.0f32;

    for i in 0..n {
        let u = u_coords[i];
        let w = w_coords[i];
        let u2 = u * u;
        sum_u4 += u2 * u2;
        sum_u3 += u2 * u;
        sum_u2 += u2;
        sum_u1 += u;
        sum_w += w;
        sum_wu += w * u;
        sum_wu2 += w * u2;
    }

    let nf = n as f32;
    let det_m = sum_u4 * (sum_u2 * nf - sum_u1 * sum_u1)
        - sum_u3 * (sum_u3 * nf - sum_u1 * sum_u2)
        + sum_u2 * (sum_u3 * sum_u1 - sum_u2 * sum_u2);

    let (fit_a, fit_b) = if det_m.abs() > 1e-4 {
        let a = (sum_wu2 * (sum_u2 * nf - sum_u1 * sum_u1)
            - sum_u3 * (sum_wu * nf - sum_u1 * sum_w)
            + sum_u2 * (sum_wu * sum_u1 - sum_u2 * sum_w)) / det_m;
        let b = (sum_u4 * (sum_wu * nf - sum_u1 * sum_w)
            - sum_wu2 * (sum_u3 * nf - sum_u1 * sum_u2)
            + sum_u2 * (sum_u3 * sum_w - sum_wu * sum_u2)) / det_m;
        (a, b)
    } else {
        (0.0, 0.0)
    };

    // Clamp curvature bounds to maintain graceful legibility
    let max_curv = 0.30 / u_half.max(2.0);
    let a_clamped = fit_a.clamp(-max_curv, max_curv);
    let b_clamped = fit_b.clamp(-0.35, 0.35);

    // If curvature is gentle or nation is compact, add a majestic subtle cartographic arch
    let curve_a = if a_clamped.abs() < 0.003 && aspect_ratio < 2.2 {
        -0.05 / u_half.max(2.0)
    } else {
        a_clamped
    };
    let curve_b = b_clamped;

    // Evaluates point on curved spine in world coordinates and tangent derivative
    let eval_curve = |u: f32| -> ([f32; 2], [f32; 2]) {
        let w = curve_a * u * u + curve_b * u;
        let dw_du = 2.0 * curve_a * u + curve_b;
        let px = cx + u * vx + w * wx;
        let py = cy + u * vy + w * wy;
        let tx = vx + dw_du * wx;
        let ty = vy + dw_du * wy;
        ([px, py], [tx, ty])
    };

    // Numerical Arc Length Sampling (32 discrete segments)
    const SAMPLES: usize = 32;
    let mut u_start = -u_half * 0.70;
    let mut u_end = u_half * 0.70;

    let p_start = eval_curve(u_start).0;
    let p_end = eval_curve(u_end).0;
    let d_x = p_end[0] - p_start[0];
    let d_y = p_end[1] - p_start[1];

    if d_x.abs() < 0.45 * d_y.abs() {
        if p_end[1] < p_start[1] {
            std::mem::swap(&mut u_start, &mut u_end);
        }
    } else if p_end[0] < p_start[0] {
        std::mem::swap(&mut u_start, &mut u_end);
    }

    let u_range = u_end - u_start;

    let mut arc_lens = [0.0f32; SAMPLES + 1];
    let mut prev_pt = eval_curve(u_start).0;
    for k in 1..=SAMPLES {
        let u = u_start + (k as f32 / SAMPLES as f32) * u_range;
        let pt = eval_curve(u).0;
        let d = ((pt[0] - prev_pt[0]).powi(2) + (pt[1] - prev_pt[1]).powi(2)).sqrt();
        arc_lens[k] = arc_lens[k - 1] + d;
        prev_pt = pt;
    }
    let total_arc_len = arc_lens[SAMPLES].max(1.0);

    let s_to_u = |s: f32| -> f32 {
        let s_clamped = s.clamp(0.0, total_arc_len);
        for k in 1..=SAMPLES {
            if arc_lens[k] >= s_clamped {
                let s_prev = arc_lens[k - 1];
                let s_next = arc_lens[k];
                let frac = if (s_next - s_prev).abs() > 1e-6 {
                    (s_clamped - s_prev) / (s_next - s_prev)
                } else {
                    0.0
                };
                let u_k_prev = u_start + ((k - 1) as f32 / SAMPLES as f32) * u_range;
                let u_k_next = u_start + (k as f32 / SAMPLES as f32) * u_range;
                return u_k_prev + frac * (u_k_next - u_k_prev);
            }
        }
        u_end
    };

    let num_chars = chars.len();
    let target_arc = total_arc_len * 0.78;

    let natural_font = (thickness * 0.16 + geo_mean * 0.05 + 0.35).clamp(0.4, 6.5);
    let max_font_for_span = (target_arc * 1.15) / (0.80 * (num_chars - 1).max(1) as f32 + 0.70);
    let world_font_size = natural_font.min(max_font_for_span).clamp(0.25, 6.0);

    let ideal_step = (target_arc - world_font_size * 0.70) / (num_chars - 1).max(1) as f32;
    let s_step = ideal_step.clamp(world_font_size * 0.85, world_font_size * 2.20);
    let total_used_arc = (num_chars - 1) as f32 * s_step;
    let s_start = ((total_arc_len - total_used_arc) * 0.5).max(0.0);

    let is_vertical = d_x.abs() < 0.45 * d_y.abs();

    let mut label_chars = Vec::with_capacity(num_chars);
    for (i, &ch) in chars.iter().enumerate() {
        let s = s_start + i as f32 * s_step;
        let u = s_to_u(s);
        let (pt, tangent) = eval_curve(u);

        let angle = if is_vertical {
            0.0
        } else {
            let mut a = tangent[1].atan2(tangent[0]);
            if a > std::f32::consts::FRAC_PI_2 {
                a -= std::f32::consts::PI;
            } else if a < -std::f32::consts::FRAC_PI_2 {
                a += std::f32::consts::PI;
            }
            a.clamp(-0.52, 0.52) // Gracefully clamped within +/- 30 degrees
        };

        label_chars.push(NationLabelChar {
            ch,
            world_pos: pt,
            angle,
        });
    }

    Some(NationLabel {
        group_key: group_key.to_string(),
        name: name.to_string(),
        chars: label_chars,
        center: [cx, cy],
        world_span: u_span,
        world_font_size,
        province_count: total_province_count,
        min_zoom,
        max_zoom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{compute_path_centroid, parse_svg_paths};
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn test_print_labels() {
        let svg_content = fs::read_to_string("Blank_Map.svg").unwrap();
        let raw_paths = parse_svg_paths(&svg_content);
        let mut provinces = Vec::new();
        for (index, p) in raw_paths.iter().enumerate() {
            let centroid = compute_path_centroid(&p.d);
            provinces.push(Province {
                id: p.id.clone(),
                index,
                name: p.id.clone(),
                group_key: String::new(),
                group_label: String::new(),
                color: [200, 200, 200],
                is_wasteland: false,
                centroid: [centroid.0, centroid.1],
                settlement: None,
                biome: crate::types::BiomeType::Grassland,
            });
        }

        let config_str = fs::read_to_string("mapchart-config-world-1450.txt").unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let mut groups = HashMap::new();
        if let Some(grps) = json_val.get("groups").and_then(|g| g.as_object()) {
            for (k, v) in grps {
                let label = v.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string();
                let paths: HashSet<String> = v.get("paths").and_then(|p| p.as_array())
                    .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect())
                    .unwrap_or_default();
                groups.insert(k.clone(), MapGroup {
                    key: k.clone(),
                    label,
                    color: [255, 255, 255],
                    paths,
                    capital_province_id: None,
                    capital_name: None,
                    capital_pos: None,
                });
            }
        }

        let labels = generate_nation_labels(&groups, &provinces, LabelAlgorithm::Curved);
        let mut sorted = labels.clone();
        sorted.sort_by(|a, b| b.world_font_size.partial_cmp(&a.world_font_size).unwrap());

        println!("\n--- TOP 15 LARGEST FONT LABELS ---");
        for l in sorted.iter().take(15) {
            println!("{:<22} | Provs: {:4} | Chars: {:2} | Span: {:5.1} | Font: {:4.2}", l.name, l.province_count, l.chars.len(), l.world_span, l.world_font_size);
        }

        println!("\n--- BOTTOM 15 SMALLEST FONT LABELS ---");
        for l in sorted.iter().rev().take(15) {
            println!("{:<22} | Provs: {:4} | Chars: {:2} | Span: {:5.1} | Font: {:4.2}", l.name, l.province_count, l.chars.len(), l.world_span, l.world_font_size);
        }

        println!("\n--- SPECIFIC INTEREST LABELS ---");
        for target in &["Tula", "Belgium", "France", "Holy Roman Empire", "England", "Poland", "Japan", "Inca Empire", "Chinese Empire", "Portugal", "Brittany", "Venice", "Navarre", "Scotland", "Ireland", "Kazan", "Lithuania", "Netherlands", "Norway", "Sweden", "Egypt"] {
            if let Some(l) = labels.iter().find(|x| x.name.eq_ignore_ascii_case(target)) {
                println!("{:<22} | Provs: {:4} | Chars: {:2} | Span: {:5.1} | Font: {:4.2}", l.name, l.province_count, l.chars.len(), l.world_span, l.world_font_size);
            }
        }
    }
}
