use super::{App, DisplayItem, SplitFocus};
use crate::countries;

impl App {
    pub fn toggle_split_view(&mut self) {
        self.split_view = !self.split_view;
        if self.split_view {
            // Entering split view - apply current search state
            if !self.search_query.is_empty() {
                self.update_split_view_for_search();
            } else {
                // Restore full country list (in case it was filtered before)
                self.country_list = self.full_country_list.clone();

                // Sync country selection with current tree view selection
                if let Some(idx) = self.state.selected() {
                    if let Some(item) = self.displayed_items.get(idx) {
                        let country_code = match item {
                            DisplayItem::CountryHeader(c) => c.clone(),
                            DisplayItem::Server(server_idx) => {
                                self.all_servers[*server_idx].exit_country.clone()
                            }
                        };
                        if let Some(pos) = self.country_list.iter().position(|c| c == &country_code)
                        {
                            self.country_state.select(Some(pos));
                        }
                    }
                }
                // Ensure we have a valid selection
                if self.country_state.selected().is_none() && !self.country_list.is_empty() {
                    self.country_state.select(Some(0));
                }
                self.update_server_list_for_selected_country();
            }
            self.split_focus = SplitFocus::Countries;
        } else {
            // Exiting split view - refresh tree view with current search state
            self.update_display_list();
            if !self.displayed_items.is_empty() && self.state.selected().is_none() {
                self.state.select(Some(0));
            }
        }
    }

    pub fn update_server_list_for_selected_country(&mut self) {
        self.server_list.clear();
        if let Some(idx) = self.country_state.selected() {
            if let Some(country_code) = self.country_list.get(idx) {
                for (i, server) in self.all_servers.iter().enumerate() {
                    if &server.exit_country == country_code {
                        self.server_list.push(i);
                    }
                }
                if !self.server_list.is_empty() {
                    self.server_state.select(Some(0));
                } else {
                    self.server_state.select(None);
                }
            }
        }
    }

    pub fn update_split_view_for_search(&mut self) {
        if self.search_query.is_empty() {
            // Restore full country list and normal behavior
            self.country_list = self.full_country_list.clone();
            if !self.country_list.is_empty() {
                if self.country_state.selected().is_none() {
                    self.country_state.select(Some(0));
                }
                self.update_server_list_for_selected_country();
            }
            return;
        }

        let query = &self.search_query;

        // 1. Find ALL matching servers with scores (regardless of country)
        let mut scored_servers = self.collect_scored_servers(query);

        // Sort by score (lower is better), then by server name
        scored_servers.sort_by(|a, b| {
            let score_cmp = a.1.cmp(&b.1);
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }
            self.all_servers[a.0].name.cmp(&self.all_servers[b.0].name)
        });

        self.server_list = scored_servers.into_iter().map(|(idx, _)| idx).collect();

        // 2. Filter and sort country list by best match score
        let mut scored_countries = self.collect_scored_countries(query);

        scored_countries.sort_by(|a, b| {
            let score_cmp = a.1.cmp(&b.1);
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }
            countries::get_country_name(&a.0).cmp(&countries::get_country_name(&b.0))
        });

        self.country_list = scored_countries.into_iter().map(|(code, _)| code).collect();

        // 3. Reset selections
        if !self.server_list.is_empty() {
            self.server_state.select(Some(0));
        } else {
            self.server_state.select(None);
        }
        if !self.country_list.is_empty() {
            self.country_state.select(Some(0));
        } else {
            self.country_state.select(None);
        }
    }

    pub fn split_next(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                let len = self.country_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.country_state.selected() {
                    Some(i) => {
                        if i >= len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.country_state.select(Some(i));
                self.update_server_list_for_selected_country();
            }
            SplitFocus::Servers => {
                let len = self.server_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.server_state.selected() {
                    Some(i) => {
                        if i >= len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.server_state.select(Some(i));
            }
        }
    }

    pub fn split_previous(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                let len = self.country_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.country_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.country_state.select(Some(i));
                self.update_server_list_for_selected_country();
            }
            SplitFocus::Servers => {
                let len = self.server_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.server_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.server_state.select(Some(i));
            }
        }
    }

    pub fn split_page_down(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                let len = self.country_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.country_state.selected() {
                    Some(i) => std::cmp::min(i + 10, len - 1),
                    None => 0,
                };
                self.country_state.select(Some(i));
                self.update_server_list_for_selected_country();
            }
            SplitFocus::Servers => {
                let len = self.server_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.server_state.selected() {
                    Some(i) => std::cmp::min(i + 10, len - 1),
                    None => 0,
                };
                self.server_state.select(Some(i));
            }
        }
    }

    pub fn split_page_up(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                let len = self.country_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.country_state.selected() {
                    Some(i) => i.saturating_sub(10),
                    None => 0,
                };
                self.country_state.select(Some(i));
                self.update_server_list_for_selected_country();
            }
            SplitFocus::Servers => {
                let len = self.server_list.len();
                if len == 0 {
                    return;
                }
                let i = match self.server_state.selected() {
                    Some(i) => i.saturating_sub(10),
                    None => 0,
                };
                self.server_state.select(Some(i));
            }
        }
    }

    pub fn split_go_to_first(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                if !self.country_list.is_empty() {
                    self.country_state.select(Some(0));
                    self.update_server_list_for_selected_country();
                }
            }
            SplitFocus::Servers => {
                if !self.server_list.is_empty() {
                    self.server_state.select(Some(0));
                }
            }
        }
    }

    pub fn split_go_to_last(&mut self) {
        match self.split_focus {
            SplitFocus::Countries => {
                if !self.country_list.is_empty() {
                    self.country_state.select(Some(self.country_list.len() - 1));
                    self.update_server_list_for_selected_country();
                }
            }
            SplitFocus::Servers => {
                if !self.server_list.is_empty() {
                    self.server_state.select(Some(self.server_list.len() - 1));
                }
            }
        }
    }

    pub fn split_switch_focus(&mut self) {
        self.split_focus = match self.split_focus {
            SplitFocus::Countries => SplitFocus::Servers,
            SplitFocus::Servers => SplitFocus::Countries,
        };
    }

    pub fn get_selected_server_idx_in_split(&self) -> Option<usize> {
        if let Some(idx) = self.server_state.selected() {
            self.server_list.get(idx).copied()
        } else {
            None
        }
    }
}
