mod line;
mod tab;

use once_cell::sync::OnceCell;
use std::cmp::{max, min};
use std::collections::BTreeMap;
use std::convert::TryInto;

use serde::{Deserialize, Serialize};
use tab::get_tab_to_focus;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;

use crate::line::tab_line;

const ARROW_SEPARATOR: &str = ">";
static SEGMENT: OnceCell<Segment> = OnceCell::new();

#[derive(Debug, Default)]
pub struct LinePart {
    part: String,
    len: usize,
    tab_index: Option<usize>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct Style {
    #[serde(default)]
    foreground: Option<PaletteColor>,
    #[serde(default)]
    background: Option<PaletteColor>,
    #[serde(default)]
    inverted: Option<bool>,
}

impl Style {
    /// Paint the given `text` in the current style.
    pub fn paint(&self, text: String) -> ansi_term::ANSIGenericString<str> {
        let mut style = match (self.foreground, self.background) {
            (Some(fg), Some(bg)) => style!(fg, bg),
            (Some(fg), None) => style!(fg),
            (None, Some(bg)) => style!(PaletteColor::default(), bg),
            (None, None) => return ansi_term::Style::default().paint("".to_string()),
        };
        if self.inverted.unwrap_or(false) {
            style = style.reverse();
        }
        style.paint(text)
    }

    /// Merge this style with another, preferring the configured values in this style if present
    /// and filling in with values from `other` when missing.
    pub fn merge_with(&self, other: &Self) -> Self {
        Style {
            foreground: self.foreground.or(other.foreground).clone(),
            background: self.background.or(other.background).clone(),
            inverted: self.inverted.or(other.inverted).clone(),
        }
    }

    pub fn inverted(mut self) -> Self {
        self.inverted = self.inverted.map(|val| !val);
        self
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct SegmentPart {
    text: String,
    #[serde(default)]
    active_style: Option<Style>,
    #[serde(default)]
    inactive_style: Option<Style>,
}

impl From<&str> for SegmentPart {
    fn from(value: &str) -> Self {
        Self {
            text: value.to_string(),
            ..Default::default()
        }
    }
}

impl SegmentPart {
    pub fn to_ansi(&self, is_active: bool) -> ansi_term::ANSIGenericString<str> {
        match (is_active, &self.active_style, &self.inactive_style) {
            (true, Some(style), _) | (false, _, Some(style)) => style.paint(self.text.clone()),
            _ => ansi_term::Style::default().paint(self.text.clone()),
        }
    }

    /// Get the currently active theme.
    pub fn current_theme(&self, is_active: bool) -> Style {
        if is_active {
            self.active_style.clone()
        } else {
            self.inactive_style.clone()
        }
        .unwrap_or_default()
    }

    /// Merge this segment part with another, preferring the config values of `self` if present and
    /// taking from `other` if not.
    pub fn merge_with(&self, other: &Self) -> Self {
        let text = if self.text.is_empty() {
            &other.text
        } else {
            &self.text
        };
        let active_style = match (&self.active_style, &other.active_style) {
            (Some(ref some), Some(ref other)) => Some(some.merge_with(other)),
            (Some(style), None) | (None, Some(style)) => Some(style.clone()),
            (None, None) => None,
        };
        let inactive_style = match (&self.inactive_style, &other.inactive_style) {
            (Some(ref some), Some(ref other)) => Some(some.merge_with(other)),
            (Some(style), None) | (None, Some(style)) => Some(style.clone()),
            (None, None) => None,
        };
        Self {
            text: text.clone(),
            active_style,
            inactive_style,
        }
    }

    /// Merge this segments style with another. Merging follows the rules of [`Style::merge()`].
    pub fn merge_with_style(mut self, styles: &AllStyles) -> Self {
        Self {
            text: self.text.clone(),
            active_style: Some(
                self.active_style
                    .map(|current| current.merge_with(&styles.active))
                    .unwrap_or(styles.active.clone()),
            ),
            inactive_style: Some(
                self.inactive_style
                    .map(|current| current.merge_with(&styles.inactive))
                    .unwrap_or(styles.inactive.clone()),
            ),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AllStyles {
    active: Style,
    inactive: Style,
}

impl AllStyles {
    pub fn merge_with(&self, other: &Self) -> Self {
        Self {
            active: self.active.merge_with(&other.active),
            inactive: self.inactive.merge_with(&other.inactive),
        }
    }

    pub fn paint(&self, msg: String, is_active: bool) -> ansi_term::ANSIGenericString<str> {
        if is_active {
            self.active.paint(msg)
        } else {
            self.inactive.paint(msg)
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SegmentConfig {
    #[serde(default)]
    global_style: Option<AllStyles>,
    #[serde(default)]
    start_separator: Option<SegmentPart>,
    #[serde(default)]
    tab_name: Option<SegmentPart>,
    #[serde(default)]
    clients_template: Option<SegmentPart>,
    #[serde(default)]
    sync_template: Option<SegmentPart>,
    #[serde(default)]
    end_separator: Option<SegmentPart>,
}

#[derive(Debug, Default)]
struct Segment {
    start_separator: SegmentPart,
    tab_name: AllStyles,
    clients_template: SegmentPart,
    sync_template: SegmentPart,
    end_separator: SegmentPart,
}

impl Segment {
    pub fn from_config(config: SegmentConfig, default_colors: &Palette) -> Self {
        let default_style = AllStyles {
            active: config
                .global_style
                .as_ref()
                .map(|style| style.active.clone())
                .unwrap_or(Style {
                    foreground: Some(default_colors.black.clone()),
                    background: Some(default_colors.green.clone()),
                    inverted: Some(false),
                }),
            inactive: config
                .global_style
                .map(|style| style.inactive)
                .unwrap_or(Style {
                    foreground: Some(default_colors.black.clone()),
                    background: Some(default_colors.fg.clone()),
                    inverted: Some(false),
                }),
        };
        let tab_style = config
            .tab_name
            .unwrap_or_else(|| SegmentPart::default())
            .merge_with_style(&default_style);

        Self {
            start_separator: config
                .start_separator
                .unwrap_or_else(|| "".into())
                .merge_with_style(&default_style),
            tab_name: AllStyles {
                active: tab_style.active_style.unwrap(),
                inactive: tab_style.inactive_style.unwrap(),
            },
            clients_template: config
                .clients_template
                .unwrap_or_else(|| " [{}]".into())
                .merge_with_style(&default_style),
            sync_template: config
                .sync_template
                .unwrap_or_else(|| " (S)".into())
                .merge_with_style(&default_style),
            end_separator: config
                .end_separator
                .unwrap_or_else(|| SegmentPart {
                    text: "".to_string(),
                    active_style: Some(Style::default().inverted()),
                    inactive_style: Some(Style::default().inverted()),
                })
                .merge_with_style(&default_style),
        }
    }

    pub fn style(&self, tab: &TabInfo, is_renaming: bool, default_palette: Palette) -> LinePart {
        let clients = tab.other_focused_clients.as_slice();
        let clients_theme = self.clients_template.current_theme(tab.active);
        let tab_name = if is_renaming {
            "Enter name...".to_string()
        } else {
            tab.name.clone()
        };

        let mut tab_parts: Vec<ansi_term::ANSIGenericString<str>> = Vec::with_capacity(20);
        tab_parts.push(self.start_separator.to_ansi(tab.active));
        tab_parts.push(self.tab_name.paint(tab_name, tab.active));
        if !clients.is_empty() {
            if let Some((before, after)) = self.clients_template.text.split_once("{}") {
                let (mut cursors, _) = tab::cursors(clients, default_palette);
                tab_parts.push(clients_theme.paint(before.to_string()));
                tab_parts.append(&mut cursors);
                tab_parts.push(clients_theme.paint(after.to_string()));
            }
        }
        tab_parts.push(self.sync_template.to_ansi(tab.active));
        tab_parts.push(self.end_separator.to_ansi(tab.active));

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
    config: BTreeMap<String, String>,
    ribbon_theme: OnceCell<Segment>,
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

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = configuration;
        set_selectable(true);
        request_permission(&[PermissionType::ReadApplicationState]);
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
                if SEGMENT.get().is_none() {
                    let segment_config: SegmentConfig = self
                        .config
                        .get("segment")
                        .and_then(|conf| serde_json::from_str(conf).ok())
                        .unwrap_or_default();
                    let segment =
                        Segment::from_config(segment_config, &self.mode_info.style.colors);
                    // TODO(hartan): Apparently this isn't being set, don't know why...
                    SEGMENT.set(segment).unwrap();
                }
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
            if let Some(ref tab) = SEGMENT.get() {
                all_tabs.push(tab.style(t, is_renaming, self.mode_info.style.colors));
            }
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
