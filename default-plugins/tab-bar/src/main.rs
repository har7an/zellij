mod line;
mod tab;

use std::cmp::{max, min};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt;

use serde::{Deserialize, Serialize};
use tab::get_tab_to_focus;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;

use crate::line::tab_line;

const ARROW_SEPARATOR: &str = ">";

#[derive(Debug, Default)]
pub struct LinePart {
    part: String,
    len: usize,
    tab_index: Option<usize>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Style {
    #[serde(default)]
    foreground: Option<PaletteColor>,
    #[serde(default)]
    background: Option<PaletteColor>,
    #[serde(default)]
    inverted: bool,
}

impl Style {
    pub fn paint(&self, text: String) -> ansi_term::ANSIGenericString<str> {
        let mut style = match (self.foreground, self.background) {
            (Some(fg), Some(bg)) => style!(fg, bg),
            (Some(fg), None) => style!(fg),
            (None, Some(bg)) => style!(PaletteColor::default(), bg),
            (None, None) => return ansi_term::Style::default().paint("".to_string()),
        };
        if self.inverted {
            style = style.reverse();
        }
        style.paint(text)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Separator {
    text: String,
    #[serde(default)]
    style: Style,
}

impl Separator {
    pub fn as_ansi(&self) -> ansi_term::ANSIGenericString<str> {
        self.style.paint(self.text.clone())
    }
}

impl fmt::Display for Separator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ansi())
    }
}

// ribbon: {
//   start: {
//     text: ">",
//     style {
//       foreground: 255,
//       background: 0,
//       inverted: true,
//     }
//   }
// }
#[derive(Debug, Default, Serialize, Deserialize)]
struct RibbonStyle {
    start: Option<Separator>,
    end: Option<Separator>,
    style: Style,
}

impl RibbonStyle {
    pub fn default_active(colors: &Palette) -> Self {
        let foreground = match colors.theme_hue {
            ThemeHue::Dark => Some(colors.black),
            ThemeHue::Light => Some(colors.white),
        };

        Self {
            start: Some(Separator {
                text: "".to_string(),
                style: Style {
                    inverted: false,
                    background: Some(colors.green),
                    foreground,
                },
            }),
            end: Some(Separator {
                text: "".to_string(),
                style: Style {
                    inverted: true,
                    background: Some(colors.green),
                    foreground,
                },
            }),
            style: Style {
                inverted: false,
                background: Some(colors.green),
                foreground,
            },
        }
    }

    pub fn default_inactive(colors: &Palette) -> Self {
        let foreground = match colors.theme_hue {
            ThemeHue::Dark => Some(colors.black),
            ThemeHue::Light => Some(colors.white),
        };

        Self {
            start: Some(Separator {
                text: "".to_string(),
                style: Style {
                    inverted: false,
                    background: Some(colors.fg),
                    foreground,
                },
            }),
            end: Some(Separator {
                text: "".to_string(),
                style: Style {
                    inverted: true,
                    background: Some(colors.fg),
                    foreground,
                },
            }),
            style: Style {
                inverted: false,
                background: Some(colors.fg),
                foreground,
            },
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ribbon {
    active: RibbonStyle,
    inactive: RibbonStyle,
}

impl Ribbon {
    pub fn new_with_palette(colors: &Palette) -> Self {
        Self {
            active: RibbonStyle::default_active(colors),
            inactive: RibbonStyle::default_inactive(colors),
        }
    }

    pub fn style(&self, tab: &TabInfo, is_renaming: bool, full_palette: Palette) -> LinePart {
        let clients = tab.other_focused_clients.as_slice();
        let tab_style = if tab.active {
            &self.active
        } else {
            &self.inactive
        };
        let space = tab_style.style.paint(" ".to_string());
        let tab_name = if is_renaming {
            "Enter name...".to_string()
        } else {
            tab.name.clone()
        };
        // TODO(hartan): "Enter Name..." when in rename mode and active tab

        let mut tab_parts: Vec<ansi_term::ANSIGenericString<str>> = Vec::with_capacity(20);

        // Start separator
        if let Some(sep) = &tab_style.start {
            tab_parts.push(sep.as_ansi().clone());
        }
        // Tab name
        tab_parts.push(space.clone());
        tab_parts.push(tab_style.style.paint(tab_name));
        tab_parts.push(space.clone());
        // Other clients in this tab
        if !clients.is_empty() {
            let (mut cursors, _) = tab::cursors(clients, full_palette);
            tab_parts.push(tab_style.style.paint("[".to_string()));
            tab_parts.append(&mut cursors);
            tab_parts.push(tab_style.style.paint("]".to_string()));
        }
        // Synced tab
        if tab.is_sync_panes_active {
            tab_parts.push(tab_style.style.paint(" (Sync)".to_string()));
        }
        // End separator
        if let Some(ref sep) = tab_style.end {
            tab_parts.push(sep.as_ansi().clone());
        }

        // Collect all pieces into one string
        let tab_text = ansi_term::ANSIGenericStrings(&tab_parts[..]);

        LinePart {
            part: tab_text.to_string(),
            len: ansi_term::unstyled_len(&tab_text),
            tab_index: Some(tab.position),
        }
    }
}

struct TabLine {
    prefix: String,
    suffix: String,
    session_name: bool,
}

impl Default for TabLine {
    fn default() -> Self {
        Self {
            prefix: " Zellij (".to_string(),
            suffix: ") ".to_string(),
            session_name: true,
        }
    }
}

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    active_tab_idx: usize,
    mode_info: ModeInfo,
    tab_line: Vec<LinePart>,
    ribbon_theme: Ribbon,
}

impl State {
    pub fn tabs_before_active(&mut self) -> Vec<&mut TabInfo> {
        let (before, _) = self.tabs.split_at_mut(self.active_tab_idx);
        before.iter_mut().collect::<Vec<_>>()
    }

    pub fn active_tab(&mut self) -> &mut TabInfo {
        self.tabs.get_mut(self.active_tab_idx).unwrap()
    }

    pub fn tabs_after_active(&mut self) -> Vec<&mut TabInfo> {
        let (_, after) = self.tabs.split_at_mut(self.active_tab_idx);
        after.iter_mut().skip(1).collect::<Vec<_>>()
    }
}

//#[derive(serde::Deserialize, serde::Serialize)]
//pub struct Separator {
//    begin: String,
//    end: String,
//}
//
//impl Separator {
//    pub fn begin<'a>(&'a self) -> &'a str {
//        self.begin.as_ref()
//    }
//
//    pub fn end<'a>(&'a self) -> &'a str {
//        self.end.as_ref()
//    }
//}
//
//impl Default for Separator {
//    fn default() -> Self {
//        Self {
//            begin: " ".to_string(),
//            end: "".to_string(),
//        }
//    }
//}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        set_selectable(false);
        subscribe(&[
            EventType::TabUpdate,
            EventType::ModeUpdate,
            EventType::Mouse,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::ModeUpdate(mode_info) => {
                if self.mode_info != mode_info {
                    should_render = true;
                }
                self.ribbon_theme = Ribbon::new_with_palette(&mode_info.style.colors);
                self.mode_info = mode_info;
            },
            Event::TabUpdate(tabs) => {
                if let Some(active_tab_index) = tabs.iter().position(|t| t.active) {
                    // tabs are indexed starting from 1 so we need to add 1
                    let active_tab_idx = active_tab_index + 1;

                    if self.active_tab_idx != active_tab_idx || self.tabs != tabs {
                        should_render = true;
                    }
                    self.active_tab_idx = active_tab_idx;
                    self.tabs = tabs;
                } else {
                    eprintln!("Could not find active tab.");
                }
            },
            Event::Mouse(me) => match me {
                Mouse::LeftClick(_, col) => {
                    let tab_to_focus = get_tab_to_focus(&self.tab_line, self.active_tab_idx, col);
                    if let Some(idx) = tab_to_focus {
                        switch_tab_to(idx.try_into().unwrap());
                    }
                },
                Mouse::ScrollUp(_) => {
                    switch_tab_to(min(self.active_tab_idx + 1, self.tabs.len()) as u32);
                },
                Mouse::ScrollDown(_) => {
                    switch_tab_to(max(self.active_tab_idx.saturating_sub(1), 1) as u32);
                },
                _ => {},
            },
            _ => {
                eprintln!("Got unrecognized event: {:?}", event);
            },
        }
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        if self.tabs.is_empty() {
            return;
        }
        let mut all_tabs: Vec<LinePart> = vec![];
        let mut active_tab_index = 0;
        for t in &mut self.tabs {
            let mut is_renaming = false;
            if t.active {
                active_tab_index = t.position;
                if self.mode_info.mode == InputMode::RenameTab {
                    is_renaming = true;
                }
            }
            let tab = self
                .ribbon_theme
                .style(t, is_renaming, self.mode_info.style.colors);
            all_tabs.push(tab);
        }
        self.tab_line = tab_line(
            self.mode_info.session_name.as_deref(),
            all_tabs,
            active_tab_index,
            cols.saturating_sub(1),
            self.mode_info.style.colors,
            self.mode_info.capabilities,
            self.mode_info.style.hide_session_name,
        );

        let output = self
            .tab_line
            .iter()
            .fold(String::new(), |output, part| output + &part.part);
        print!("{}", output);
    }
}
