use super::{App, DisplayItem};
use crate::countries;
use crate::models::LogicalServer;

impl App {
    pub fn fuzzy_match(text: &str, query: &str) -> bool {
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut text_chars = text_lower.chars();
        for q_char in query_lower.chars() {
            if !text_chars.any(|t_char| t_char == q_char) {
                return false;
            }
        }
        true
    }

    /// Returns a match score for ranking search results.
    /// Lower score = higher priority. None = no match.
    /// 0 = exact prefix match (text starts with query)
    /// 1 = word boundary prefix match (a word in text starts with query)
    /// 2 = contains match (query appears somewhere in text)
    /// 3 = fuzzy match (all chars present in order)
    pub fn match_score(text: &str, query: &str) -> Option<u8> {
        if query.is_empty() {
            return Some(0);
        }

        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        // Score 0: Exact prefix match
        if text_lower.starts_with(&query_lower) {
            return Some(0);
        }

        // Score 1: Word boundary prefix match (a word starts with query)
        for word in text_lower.split_whitespace() {
            if word.starts_with(&query_lower) {
                return Some(1);
            }
        }
        // Also check after common separators like '-', '_'
        for part in text_lower.split(['-', '_', '#']) {
            if part.starts_with(&query_lower) {
                return Some(1);
            }
        }

        // Score 2: Contains match
        if text_lower.contains(&query_lower) {
            return Some(2);
        }

        // Score 3: Fuzzy match
        if Self::fuzzy_match(text, query) {
            return Some(3);
        }

        None
    }

    pub(crate) fn score_server_for_query(&self, server: &LogicalServer, query: &str) -> Option<u8> {
        let country_name = countries::get_country_name(&server.exit_country);

        // Get best score from country, city, or server name
        let country_score = Self::match_score(&country_name, query)
            .min(Self::match_score(&server.exit_country, query));
        let city_score = Self::match_score(&server.city, query);
        let name_score = Self::match_score(&server.name, query);

        // Take the best (lowest) score
        [country_score, city_score, name_score]
            .into_iter()
            .flatten()
            .min()
    }

    pub(crate) fn collect_scored_servers(&self, query: &str) -> Vec<(usize, u8)> {
        let mut scored_servers: Vec<(usize, u8)> = Vec::new();

        for (i, server) in self.all_servers.iter().enumerate() {
            if let Some(score) = self.score_server_for_query(server, query) {
                scored_servers.push((i, score));
            }
        }

        scored_servers
    }

    pub(crate) fn collect_scored_countries(&self, query: &str) -> Vec<(String, u8)> {
        self.full_country_list
            .iter()
            .filter_map(|code| {
                let name = countries::get_country_name(code);
                let code_score = Self::match_score(code, query);
                let name_score = Self::match_score(&name, query);
                let best = code_score.into_iter().chain(name_score).min();
                best.map(|score| (code.clone(), score))
            })
            .collect()
    }

    pub fn update_display_list(&mut self) {
        self.displayed_items.clear();
        let query = &self.search_query;
        let is_searching = !query.is_empty();

        if is_searching {
            let mut scored_servers = self.collect_scored_servers(query);

            // Sort by score (lower is better), then by country name, then by server name
            scored_servers.sort_by(|a, b| {
                let score_cmp = a.1.cmp(&b.1);
                if score_cmp != std::cmp::Ordering::Equal {
                    return score_cmp;
                }
                // Same score: sort by country, then server name
                let server_a = &self.all_servers[a.0];
                let server_b = &self.all_servers[b.0];
                let country_cmp = countries::get_country_name(&server_a.exit_country)
                    .cmp(&countries::get_country_name(&server_b.exit_country));
                if country_cmp != std::cmp::Ordering::Equal {
                    return country_cmp;
                }
                server_a.name.cmp(&server_b.name)
            });

            // Build display list, grouping by country
            let mut current_country = String::new();
            for (server_idx, _score) in scored_servers {
                let server = &self.all_servers[server_idx];
                if server.exit_country != current_country {
                    current_country = server.exit_country.clone();
                    self.displayed_items
                        .push(DisplayItem::CountryHeader(current_country.clone()));
                }
                self.displayed_items.push(DisplayItem::Server(server_idx));
            }
        } else {
            // Normal mode: show all countries, expand selected ones
            let mut current_country = String::new();
            let mut country_header_pushed = false;

            for (i, server) in self.all_servers.iter().enumerate() {
                if server.exit_country != current_country {
                    current_country = server.exit_country.clone();
                    country_header_pushed = false;
                }

                if !country_header_pushed {
                    self.displayed_items
                        .push(DisplayItem::CountryHeader(current_country.clone()));
                    country_header_pushed = true;
                }
                if self.expanded_countries.contains(&current_country) {
                    self.displayed_items.push(DisplayItem::Server(i));
                }
            }
        }
    }
}
