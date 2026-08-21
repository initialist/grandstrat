use crate::camera::Camera;
use crate::types::{BiomeType, MapGroup, Province, Settlement, SettlementInfo, SettlementTier};
use eframe::egui;
use std::collections::HashMap;

pub struct SettlementRegistry {
    pub settlements: Vec<Settlement>,
}

impl SettlementRegistry {
    pub fn build(provinces: &mut [Province], groups: &mut HashMap<String, MapGroup>) -> Self {
        // ----------------------------------------------------------------
        // Predefined Historical 1450 Capitals & Major Cities
        // (Clean Title Case / Mixed Case)
        // ----------------------------------------------------------------
        let tier1_capitals: HashMap<&'static str, &'static str> = [
            // Western & Central Europe
            ("paris", "Paris"),
            ("london", "London"),
            ("rome", "Rome"),
            ("roma", "Rome"),
            ("constantinople", "Constantinople"),
            ("venice", "Venice"),
            ("venezia", "Venice"),
            ("genoa", "Genoa"),
            ("genova", "Genoa"),
            ("lisbon", "Lisbon"),
            ("lisboa", "Lisbon"),
            ("toledo", "Toledo"),
            ("madrid", "Madrid"),
            ("wien", "Vienna"),
            ("vienna", "Vienna"),
            ("krakow", "Kraków"),
            ("cracow", "Kraków"),
            ("prague", "Prague"),
            ("praha", "Prague"),
            ("moscow", "Moscow"),
            ("moskva", "Moscow"),
            ("novgorod", "Novgorod"),
            ("stockholm", "Stockholm"),
            ("copenhagen", "Copenhagen"),
            ("kobenhavn", "Copenhagen"),
            ("edinburgh", "Edinburgh"),
            ("dublin", "Dublin"),
            ("naples", "Naples"),
            ("napoli", "Naples"),
            ("florence", "Florence"),
            ("firenze", "Florence"),
            ("milan", "Milan"),
            ("milano", "Milan"),
            ("bruges", "Bruges"),
            ("amsterdam", "Amsterdam"),
            ("cologne", "Cologne"),
            ("koln", "Cologne"),
            ("nuremberg", "Nuremberg"),
            ("nurnberg", "Nuremberg"),
            ("ragusa", "Ragusa"),
            ("dubrovnik", "Ragusa"),
            ("athens", "Athens"),
            ("trebizond", "Trebizond"),
            ("trabzon", "Trebizond"),
            ("antwerp", "Antwerp"),
            ("valencia", "Valencia"),
            ("seville", "Seville"),
            ("sevilla", "Seville"),
            ("granada", "Granada"),
            ("bordeaux", "Bordeaux"),
            ("marseille", "Marseille"),
            ("lyon", "Lyon"),
            ("frankfurt", "Frankfurt"),
            ("munich", "Munich"),
            ("munchen", "Munich"),
            ("budapest", "Buda"),
            ("buda", "Buda"),
            ("warsaw", "Warsaw"),
            ("warszawa", "Warsaw"),
            ("vilnius", "Vilnius"),
            ("riga", "Riga"),
            ("reval", "Reval"),
            ("tallinn", "Reval"),
            ("bern", "Bern"),
            // Middle East & North Africa
            ("cairo", "Cairo"),
            ("al_qahirah", "Cairo"),
            ("alexandria", "Alexandria"),
            ("damascus", "Damascus"),
            ("dimashq", "Damascus"),
            ("aleppo", "Aleppo"),
            ("halab", "Aleppo"),
            ("baghdad", "Baghdad"),
            ("jerusalem", "Jerusalem"),
            ("al_quds", "Jerusalem"),
            ("mecca", "Mecca"),
            ("makkah", "Mecca"),
            ("medina", "Medina"),
            ("tabriz", "Tabriz"),
            ("isfahan", "Isfahan"),
            ("shiraz", "Shiraz"),
            ("samarkand", "Samarkand"),
            ("bukhara", "Bukhara"),
            ("herat", "Herat"),
            ("tunis", "Tunis"),
            ("algiers", "Algiers"),
            ("al_jazair", "Algiers"),
            ("fez", "Fez"),
            ("marrakesh", "Marrakesh"),
            ("tripoli", "Tripoli"),
            ("basra", "Basra"),
            ("muscat", "Muscat"),
            ("aden", "Aden"),
            ("sanaa", "Sana'a"),
            ("tehran", "Tehran"),
            ("baku", "Baku"),
            // Asia
            ("beijing", "Beijing"),
            ("peking", "Beijing"),
            ("nanjing", "Nanjing"),
            ("nanking", "Nanjing"),
            ("hangzhou", "Hangzhou"),
            ("guangzhou", "Guangzhou"),
            ("canton", "Guangzhou"),
            ("changan", "Chang'an"),
            ("xian", "Xi'an"),
            ("chengdu", "Chengdu"),
            ("luoyang", "Luoyang"),
            ("kyoto", "Kyoto"),
            ("kamakura", "Kamakura"),
            ("osaka", "Osaka"),
            ("edo", "Edo"),
            ("tokyo", "Edo"),
            ("seoul", "Hanseong"),
            ("hanseong", "Hanseong"),
            ("pyongyang", "Pyongyang"),
            ("delhi", "Delhi"),
            ("agra", "Agra"),
            ("vijayanagara", "Vijayanagara"),
            ("hampi", "Vijayanagara"),
            ("calicut", "Calicut"),
            ("kozhikode", "Calicut"),
            ("gaur", "Gaur"),
            ("chittagong", "Chittagong"),
            ("ava", "Ava"),
            ("ayutthaya", "Ayutthaya"),
            ("angkor", "Angkor"),
            ("malacca", "Malacca"),
            ("melaka", "Malacca"),
            ("brunei", "Brunei"),
            ("majapahit", "Majapahit"),
            ("samudera", "Samudera"),
            ("lahore", "Lahore"),
            ("dhaka", "Dhaka"),
            ("lhasa", "Lhasa"),
            ("kashgar", "Kashgar"),
            ("tashkent", "Tashkent"),
            // Americas
            ("tenochtitlan", "Tenochtitlan"),
            ("mexico", "Tenochtitlan"),
            ("texcoco", "Texcoco"),
            ("tlaxcala", "Tlaxcala"),
            ("cusco", "Cusco"),
            ("cuzco", "Cusco"),
            ("chan_chan", "Chan Chan"),
            ("tiwanaku", "Tiwanaku"),
            ("cahokia", "Cahokia"),
            ("chichen_itza", "Chichén Itzá"),
            ("tikal", "Tikal"),
            ("quito", "Quito"),
            // Africa
            ("gao", "Gao"),
            ("timbuktu", "Timbuktu"),
            ("jenne", "Djenné"),
            ("djenne", "Djenné"),
            ("niani", "Niani"),
            ("benin", "Benin"),
            ("kano", "Kano"),
            ("ife", "Ife"),
            ("great_zimbabwe", "Great Zimbabwe"),
            ("kilwa", "Kilwa"),
            ("mombasa", "Mombasa"),
            ("mogadishu", "Mogadishu"),
            ("gondar", "Gondar"),
            ("axum", "Axum"),
            ("maravi", "Maravi"),
            ("sofala", "Sofala"),
        ]
        .into_iter()
        .collect();

        let tier2_cities: HashMap<&'static str, &'static str> = [
            ("rouen", "Rouen"),
            ("nantes", "Nantes"),
            ("toulouse", "Toulouse"),
            ("dijon", "Dijon"),
            ("strasbourg", "Strasbourg"),
            ("orleans", "Orléans"),
            ("rennes", "Rennes"),
            ("reims", "Reims"),
            ("york", "York"),
            ("bristol", "Bristol"),
            ("norwich", "Norwich"),
            ("oxford", "Oxford"),
            ("cambridge", "Cambridge"),
            ("newcastle", "Newcastle"),
            ("aberdeen", "Aberdeen"),
            ("barcelona", "Barcelona"),
            ("zaragoza", "Zaragoza"),
            ("cordoba", "Córdoba"),
            ("salamanca", "Salamanca"),
            ("porto", "Porto"),
            ("coimbra", "Coimbra"),
            ("hamburg", "Hamburg"),
            ("lubeck", "Lübeck"),
            ("bremen", "Bremen"),
            ("augsburg", "Augsburg"),
            ("ulm", "Ulm"),
            ("leipzig", "Leipzig"),
            ("dresden", "Dresden"),
            ("breslau", "Breslau"),
            ("wroclaw", "Breslau"),
            ("gdansk", "Gdańsk"),
            ("danzig", "Gdańsk"),
            ("torun", "Toruń"),
            ("lublin", "Lublin"),
            ("poznan", "Poznań"),
            ("lviv", "Lviv"),
            ("kiev", "Kyiv"),
            ("kyiv", "Kyiv"),
            ("pskov", "Pskov"),
            ("tver", "Tver"),
            ("smolensk", "Smolensk"),
            ("ryazan", "Ryazan"),
            ("vladimir", "Vladimir"),
            ("yaroslavl", "Yaroslavl"),
            ("kazan", "Kazan"),
            ("astrakhan", "Astrakhan"),
            ("verona", "Verona"),
            ("padua", "Padua"),
            ("padova", "Padua"),
            ("bologna", "Bologna"),
            ("pisa", "Pisa"),
            ("siena", "Siena"),
            ("lucca", "Lucca"),
            ("palermo", "Palermo"),
            ("catania", "Catania"),
            ("thessaloniki", "Thessaloniki"),
            ("salonica", "Thessaloniki"),
            ("adrianople", "Adrianople"),
            ("edirne", "Adrianople"),
            ("bursa", "Bursa"),
            ("konya", "Konya"),
            ("sinop", "Sinop"),
            ("antioch", "Antioch"),
            ("antakya", "Antioch"),
            ("beirut", "Beirut"),
            ("acre", "Acre"),
            ("gaza", "Gaza"),
            ("hama", "Hama"),
            ("homs", "Homs"),
            ("mosul", "Mosul"),
            ("yazd", "Yazd"),
            ("kerman", "Kerman"),
            ("hamadan", "Hamadan"),
            ("nishapur", "Nishapur"),
            ("merv", "Merv"),
            ("khiva", "Khiva"),
            ("balkh", "Balkh"),
            ("kabul", "Kabul"),
            ("kandahar", "Kandahar"),
            ("jaipur", "Jaipur"),
            ("udaipur", "Udaipur"),
            ("ahmedabad", "Ahmedabad"),
            ("surat", "Surat"),
            ("patna", "Patna"),
            ("varanasi", "Varanasi"),
            ("benares", "Varanasi"),
            ("mumbai", "Mumbai"),
            ("hyderabad", "Hyderabad"),
            ("wuhan", "Wuhan"),
            ("suzhou", "Suzhou"),
            ("fuzhou", "Fuzhou"),
            ("chongqing", "Chongqing"),
            ("kaifeng", "Kaifeng"),
            ("taiyuan", "Taiyuan"),
            ("jinan", "Jinan"),
            ("nagasaki", "Nagasaki"),
            ("sakai", "Sakai"),
            ("sendai", "Sendai"),
            ("kagoshima", "Kagoshima"),
            ("kanazawa", "Kanazawa"),
            ("nagoya", "Nagoya"),
            ("hakata", "Hakata"),
            ("fukuoka", "Hakata"),
            ("inverness", "Inverness"),
            ("cardiff", "Cardiff"),
            ("stirling", "Stirling"),
            ("galway", "Galway"),
            ("cork", "Cork"),
            ("limerick", "Limerick"),
            ("waterford", "Waterford"),
        ]
        .into_iter()
        .collect();

        // 1. Assign Biomes and initial settlements to provinces
        for p in provinces.iter_mut() {
            let cx = p.centroid[0];
            let cy = p.centroid[1];

            // Geographic Biome Assignment
            p.biome = if cy < 70.0 {
                BiomeType::Tundra
            } else if cy < 130.0 {
                BiomeType::Taiga
            } else if (cy >= 210.0 && cy <= 310.0 && cx >= 450.0 && cx <= 650.0) // Sahara
                   || (cy >= 230.0 && cy <= 310.0 && cx >= 650.0 && cx <= 730.0) // Arabia
                   || (cy >= 180.0 && cy <= 240.0 && cx >= 800.0 && cx <= 940.0) // Gobi / Taklamakan
                   || (cy >= 430.0 && cy <= 540.0 && cx >= 950.0 && cx <= 1100.0) { // Outback
                BiomeType::Desert
            } else if (cy >= 280.0 && cy <= 430.0 && cx >= 240.0 && cx <= 400.0) // Amazon
                   || (cy >= 310.0 && cy <= 440.0 && cx >= 550.0 && cx <= 700.0) // Congo
                   || (cy >= 290.0 && cy <= 420.0 && cx >= 880.0 && cx <= 1080.0) { // Sunda / SE Asia
                BiomeType::Jungle
            } else if cy >= 130.0 && cy <= 200.0 && cx >= 680.0 && cx <= 950.0 {
                BiomeType::Steppe
            } else if (cy >= 130.0 && cy <= 220.0 && cx >= 460.0 && cx <= 680.0) // Europe
                   || (cy >= 140.0 && cy <= 240.0 && cx >= 180.0 && cx <= 320.0) // North America
                   || (cy >= 160.0 && cy <= 260.0 && cx >= 900.0 && cx <= 1060.0) { // East Asia
                BiomeType::Forest
            } else {
                BiomeType::Grassland
            };

            let clean_id = p.id.to_lowercase().replace('-', "_");

            if let Some(&name) = tier1_capitals.get(clean_id.as_str()) {
                p.settlement = Some(SettlementInfo {
                    name: name.to_string(),
                    tier: SettlementTier::Capital,
                    is_capital: true,
                });
            } else if let Some(&name) = tier2_cities.get(clean_id.as_str()) {
                p.settlement = Some(SettlementInfo {
                    name: name.to_string(),
                    tier: SettlementTier::City,
                    is_capital: false,
                });
            }
        }

        // 2. Guarantee that EVERY MapGroup has at least ONE Capital
        let mut group_paths_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, p) in provinces.iter().enumerate() {
            if !p.group_key.is_empty() && !p.is_wasteland {
                group_paths_map.entry(p.group_key.clone()).or_default().push(idx);
            }
        }

        for (g_key, p_indices) in group_paths_map {
            if p_indices.is_empty() {
                continue;
            }

            // Check if group already has a predefined capital
            let mut chosen_cap_idx = None;
            for &idx in &p_indices {
                if let Some(s) = &provinces[idx].settlement {
                    if s.tier == SettlementTier::Capital {
                        chosen_cap_idx = Some(idx);
                        break;
                    }
                }
            }

            // If not, check if it has a Tier 2 city to promote to capital
            if chosen_cap_idx.is_none() {
                for &idx in &p_indices {
                    if provinces[idx].settlement.is_some() {
                        chosen_cap_idx = Some(idx);
                        break;
                    }
                }
            }

            // If still none, pick the most central province in the group
            if chosen_cap_idx.is_none() {
                let mut sum_x = 0.0f32;
                let mut sum_y = 0.0f32;
                for &idx in &p_indices {
                    sum_x += provinces[idx].centroid[0];
                    sum_y += provinces[idx].centroid[1];
                }
                let avg_x = sum_x / p_indices.len() as f32;
                let avg_y = sum_y / p_indices.len() as f32;

                let mut best_idx = p_indices[0];
                let mut best_dist_sq = f32::INFINITY;
                for &idx in &p_indices {
                    let dx = provinces[idx].centroid[0] - avg_x;
                    let dy = provinces[idx].centroid[1] - avg_y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < best_dist_sq {
                        best_dist_sq = dist_sq;
                        best_idx = idx;
                    }
                }
                chosen_cap_idx = Some(best_idx);
            }

            if let Some(cap_idx) = chosen_cap_idx {
                let p = &mut provinces[cap_idx];
                let cap_name = if let Some(s) = &p.settlement {
                    s.name.clone()
                } else {
                    let formatted = format_location_name(&p.id);
                    p.settlement = Some(SettlementInfo {
                        name: formatted.clone(),
                        tier: SettlementTier::Capital,
                        is_capital: true,
                    });
                    formatted
                };

                if let Some(g) = groups.get_mut(&g_key) {
                    g.capital_province_id = Some(p.id.clone());
                    g.capital_name = Some(cap_name);
                    g.capital_pos = Some(p.centroid);
                }
            }
        }

        // 3. Collect all registered settlements for rendering
        let mut settlements = Vec::new();
        for (idx, p) in provinces.iter().enumerate() {
            if let Some(s) = &p.settlement {
                settlements.push(Settlement {
                    name: s.name.clone(),
                    province_id: p.id.clone(),
                    province_index: idx,
                    world_pos: p.centroid,
                    tier: s.tier,
                    group_key: p.group_key.clone(),
                    is_coastal: false,
                });
            }
        }

        // Sort so Tier 1 Capitals take top precedence in collision queries
        settlements.sort_by(|a, b| {
            let rank = |t: SettlementTier| match t {
                SettlementTier::Capital => 2,
                SettlementTier::City => 1,
                SettlementTier::Town => 0,
            };
            rank(b.tier).cmp(&rank(a.tier))
        });

        Self { settlements }
    }

    /// Deterministic World-Space LOD Querying (Zero horizontal panning jitter)
    pub fn get_visible<'a>(
        &'a self,
        camera: &Camera,
        viewport_rect: egui::Rect,
    ) -> Vec<&'a Settlement> {
        let zoom = camera.zoom;
        let mut visible = Vec::new();
        let mut chosen_world_positions: Vec<[f32; 2]> = Vec::new();

        // Fixed minimum world-space distance between visible settlements
        let min_world_dist = if zoom < 2.0 {
            12.0f32
        } else if zoom < 4.5 {
            6.0f32
        } else {
            3.0f32
        };
        let min_world_dist_sq = min_world_dist * min_world_dist;

        for s in &self.settlements {
            match s.tier {
                SettlementTier::Capital => {
                    if zoom < 0.85 {
                        continue;
                    }
                }
                SettlementTier::City => {
                    if zoom < 2.8 {
                        continue;
                    }
                }
                SettlementTier::Town => {
                    continue;
                }
            }

            // Screen-space frustum check
            let screen_pt = camera.world_to_screen(s.world_pos[0], s.world_pos[1]);
            let pos = egui::pos2(screen_pt[0], screen_pt[1]);

            if !viewport_rect.expand(60.0).contains(pos) {
                continue;
            }

            // Guaranteed Capitals always render when zoomed in >= 2.0
            if s.tier != SettlementTier::Capital || zoom < 2.0 {
                // Fixed World-Space distance rejection (deterministic, zero panning flicker)
                let mut too_close = false;
                for &other_pos in &chosen_world_positions {
                    let dx = s.world_pos[0] - other_pos[0];
                    let dy = s.world_pos[1] - other_pos[1];
                    if dx * dx + dy * dy < min_world_dist_sq {
                        too_close = true;
                        break;
                    }
                }
                if too_close {
                    continue;
                }
            }

            chosen_world_positions.push(s.world_pos);
            visible.push(s);
        }

        visible
    }
}

pub fn format_location_name(raw_id: &str) -> String {
    let unescaped = raw_id.replace('_', " ");
    let mut result = String::with_capacity(unescaped.len());
    let mut capitalize_next = true;

    for ch in unescaped.chars() {
        if ch == ' ' || ch == '-' {
            result.push(ch);
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}
