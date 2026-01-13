use super::{App, DisplayItem};

const PAGE_JUMP: usize = 10;

impl App {
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.displayed_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.displayed_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn page_down(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let next = i + PAGE_JUMP;
                if next >= self.displayed_items.len() {
                    self.displayed_items.len() - 1
                } else {
                    next
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn page_up(&mut self) {
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(PAGE_JUMP),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn go_to_first(&mut self) {
        if !self.displayed_items.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn go_to_last(&mut self) {
        if !self.displayed_items.is_empty() {
            self.state.select(Some(self.displayed_items.len() - 1));
        }
    }

    pub fn toggle_current_selection(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(item) = self.displayed_items.get(idx).cloned() {
                match item {
                    DisplayItem::CountryHeader(country) => {
                        if self.expanded_countries.contains(&country) {
                            self.expanded_countries.remove(&country);
                        } else {
                            self.expanded_countries.insert(country);
                        }
                        self.update_display_list();
                    }
                    DisplayItem::Server(_) => {
                        // handled by connect_to_selected usually
                    }
                }
            }
        }
    }

    pub fn expand_selected(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(DisplayItem::CountryHeader(country)) =
                self.displayed_items.get(idx).cloned()
            {
                if !self.expanded_countries.contains(&country) {
                    self.expanded_countries.insert(country);
                    self.update_display_list();
                }
            }
        }
    }

    pub fn collapse_selected(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(item) = self.displayed_items.get(idx).cloned() {
                match item {
                    DisplayItem::CountryHeader(country) => {
                        if self.expanded_countries.contains(&country) {
                            self.expanded_countries.remove(&country);
                            self.update_display_list();
                        }
                    }
                    DisplayItem::Server(server_idx) => {
                        let country = self.all_servers[server_idx].exit_country.clone();
                        if self.expanded_countries.contains(&country) {
                            self.expanded_countries.remove(&country);
                            self.update_display_list();

                            // Find the header index and select it
                            if let Some(header_pos) = self.displayed_items.iter().position(|it| {
                                if let DisplayItem::CountryHeader(c) = it {
                                    c == &country
                                } else {
                                    false
                                }
                            }) {
                                self.state.select(Some(header_pos));
                            }
                        }
                    }
                }
            }
        }
    }
}
