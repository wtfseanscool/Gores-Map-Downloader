//! Filtering and sorting logic

use super::App;
use crate::types::*;

impl App {
    pub fn apply_filters(&mut self) {
        let query = self.search_query.trim();
        let query_lower = query.to_lowercase();
        let is_empty = query.is_empty();

        // Save sort when starting to search, clear sort to use search relevance
        if !is_empty && self.saved_sort.is_none() {
            self.saved_sort = Some((self.sort_column, self.sort_direction));
            self.sort_column = None; // Use search sort by default
        }
        // Restore sort when clearing search
        if is_empty {
            if let Some((col, dir)) = self.saved_sort.take() {
                // Only restore if user didn't manually set a sort during search
                if self.sort_column.is_none() {
                    self.sort_column = col;
                    self.sort_direction = dir;
                }
            }
        }

        let mut scored: Vec<(usize, u8)> = self
            .maps
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                // Downloaded filter - check actual file existence
                match self.filter_downloaded {
                    1 => {
                        let path = self.download_path.join(format!("{}.map", m.name));
                        if !path.exists() {
                            return None;
                        }
                    }
                    2 => {
                        let path = self.download_path.join(format!("{}.map", m.name));
                        if path.exists() {
                            return None;
                        }
                    }
                    _ => {}
                }

                // Year filter
                if self.year_mode_range {
                    if let Some((min_year, max_year)) = self.year_range {
                        let map_year = m
                            .release_date
                            .split('-')
                            .next()
                            .and_then(|y| y.parse::<i32>().ok());
                        if let Some(year) = map_year {
                            if year < min_year || year > max_year {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                } else {
                    let map_year = m
                        .release_date
                        .split('-')
                        .next()
                        .and_then(|y| y.parse::<i32>().ok());
                    if let Some(year) = map_year {
                        if !self.filter_years.contains(&year) {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }

                // Category filter
                if let Some(cat_idx) = Self::category_index(&m.category) {
                    if self.category_mode_range {
                        if cat_idx <= 4 {
                            if (cat_idx as u8) < self.category_range.0
                                || (cat_idx as u8) > self.category_range.1
                            {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    } else {
                        if !self.filter_categories[cat_idx] {
                            return None;
                        }
                    }
                }

                // Stars filter
                let stars = m.stars as u8;
                if self.stars_mode_range {
                    if stars < self.stars_range.0 || stars > self.stars_range.1 {
                        return None;
                    }
                } else if stars >= 1 && stars <= 5 && !self.filter_stars[(stars - 1) as usize] {
                    return None;
                }

                // Search filter with priority scoring
                if query.is_empty() {
                    return Some((i, 4));
                }

                if m.name.contains(query) {
                    return Some((i, 0));
                }
                if m.author.contains(query) {
                    return Some((i, 1));
                }
                if m.name.to_lowercase().contains(&query_lower) {
                    return Some((i, 2));
                }
                if m.author.to_lowercase().contains(&query_lower) {
                    return Some((i, 3));
                }
                None
            })
            .collect();

        scored.sort_by_key(|(_, priority)| *priority);
        self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();

        // Apply column sorting
        if let Some(col) = self.sort_column {
            let maps = &self.maps;
            let dir = self.sort_direction;
            self.filtered_indices.sort_by(|&a, &b| {
                let cmp = match col {
                    SortColumn::Name => maps[a]
                        .name
                        .to_lowercase()
                        .cmp(&maps[b].name.to_lowercase()),
                    SortColumn::Category => {
                        let ca = Self::category_index(&maps[a].category).unwrap_or(99);
                        let cb = Self::category_index(&maps[b].category).unwrap_or(99);
                        ca.cmp(&cb)
                    }
                    SortColumn::Stars => maps[a].stars.cmp(&maps[b].stars),
                    SortColumn::Points => maps[a].points.cmp(&maps[b].points),
                    SortColumn::Author => maps[a]
                        .author
                        .to_lowercase()
                        .cmp(&maps[b].author.to_lowercase()),
                    SortColumn::ReleaseDate => {
                        let a_valid = maps[a].release_date.len() >= 4
                            && maps[a]
                                .release_date
                                .chars()
                                .take(4)
                                .all(|c| c.is_ascii_digit());
                        let b_valid = maps[b].release_date.len() >= 4
                            && maps[b]
                                .release_date
                                .chars()
                                .take(4)
                                .all(|c| c.is_ascii_digit());
                        match (a_valid, b_valid) {
                            (false, true) => std::cmp::Ordering::Less,
                            (true, false) => std::cmp::Ordering::Greater,
                            _ => maps[a].release_date.cmp(&maps[b].release_date),
                        }
                    }
                };
                if dir == SortDirection::Descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        self.build_scroll_index();
    }

    pub fn build_scroll_index(&mut self) {
        self.scroll_index_markers.clear();

        if self.filtered_indices.is_empty() {
            return;
        }

        let maps = &self.maps;
        let indices = &self.filtered_indices;

        match self.sort_column {
            Some(SortColumn::Name) | Some(SortColumn::Author) => {
                let get_char = |idx: usize| -> char {
                    let s = if self.sort_column == Some(SortColumn::Name) {
                        &maps[idx].name
                    } else {
                        &maps[idx].author
                    };
                    s.chars().next().unwrap_or('?').to_ascii_uppercase()
                };

                let mut current_char = '\0';
                for (row_idx, &map_idx) in indices.iter().enumerate() {
                    let c = get_char(map_idx);
                    if c != current_char {
                        current_char = c;
                        self.scroll_index_markers.push(ScrollIndexMarker {
                            label: c.to_string(),
                            row_index: row_idx,
                        });
                    }
                }
            }
            Some(SortColumn::Category) => {
                let mut current_cat = "";
                for (row_idx, &map_idx) in indices.iter().enumerate() {
                    let cat = &maps[map_idx].category;
                    if cat != current_cat {
                        current_cat = cat;
                        let label = match cat.as_str() {
                            "Easy" => "EZ",
                            "Main" => "MN",
                            "Hard" => "HD",
                            "Insane" => "IN",
                            "Extreme" => "EX",
                            "Solo" => "SO",
                            "Mod" => "MD",
                            "Extra" => "XT",
                            _ => &cat[..2.min(cat.len())],
                        };
                        self.scroll_index_markers.push(ScrollIndexMarker {
                            label: label.to_string(),
                            row_index: row_idx,
                        });
                    }
                }
            }
            Some(SortColumn::Stars) => {
                let mut current_stars = -1;
                for (row_idx, &map_idx) in indices.iter().enumerate() {
                    let stars = maps[map_idx].stars;
                    if stars != current_stars {
                        current_stars = stars;
                        self.scroll_index_markers.push(ScrollIndexMarker {
                            label: format!("{}★", stars),
                            row_index: row_idx,
                        });
                    }
                }
            }
            Some(SortColumn::Points) => {
                if indices.is_empty() {
                    return;
                }

                let mut points: Vec<i32> = indices.iter().map(|&i| maps[i].points).collect();
                points.sort();
                let min_pts = points[0];
                let max_pts = points[points.len() - 1];

                let breakpoints: Vec<i32> = if max_pts - min_pts < 20 {
                    vec![min_pts, max_pts]
                } else {
                    let q1 = points[points.len() / 4];
                    let q2 = points[points.len() / 2];
                    let q3 = points[3 * points.len() / 4];
                    let mut bp = vec![min_pts];
                    if q1 > min_pts {
                        bp.push(q1);
                    }
                    if q2 > q1 {
                        bp.push(q2);
                    }
                    if q3 > q2 {
                        bp.push(q3);
                    }
                    if max_pts > q3 {
                        bp.push(max_pts);
                    }
                    bp
                };

                let mut bp_idx = 0;
                for (row_idx, &map_idx) in indices.iter().enumerate() {
                    let pts = maps[map_idx].points;
                    while bp_idx < breakpoints.len() && pts >= breakpoints[bp_idx] {
                        self.scroll_index_markers.push(ScrollIndexMarker {
                            label: format!("{}", breakpoints[bp_idx]),
                            row_index: row_idx,
                        });
                        bp_idx += 1;
                    }
                }
            }
            Some(SortColumn::ReleaseDate) => {
                let mut current_year = "";
                for (row_idx, &map_idx) in indices.iter().enumerate() {
                    let date = &maps[map_idx].release_date;
                    let year =
                        if date.len() >= 4 && date.chars().take(4).all(|c| c.is_ascii_digit()) {
                            &date[2..4]
                        } else {
                            "NA"
                        };
                    if year != current_year {
                        current_year = year;
                        self.scroll_index_markers.push(ScrollIndexMarker {
                            label: if year == "NA" {
                                "N/A".to_string()
                            } else {
                                format!("'{}", year)
                            },
                            row_index: row_idx,
                        });
                    }
                }
            }
            None => {}
        }
    }
}
