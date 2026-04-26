pub struct Shortcut {
    pub key: &'static str,
    pub desc: &'static str,
}

impl Shortcut {
    pub const fn new(key: &'static str, desc: &'static str) -> Self {
        Self { key, desc }
    }
}

pub fn get_shortcuts(mode: &str, tab_name: &str, has_suggestions: bool) -> Vec<Shortcut> {
    match mode {
        "List" => {
            let mut shortcuts = vec![
                Shortcut::new("q", "Quit"),
                Shortcut::new("1-6/Tab/hl", "Tabs"),
                Shortcut::new("j/k/g/G", "Nav"),
                Shortcut::new("s", "Stage"),
                Shortcut::new("a/i", "Add"),
                Shortcut::new("e/Ent", "Edit"),
                Shortcut::new("d", "Del"),
                Shortcut::new("m", "Move"),
                Shortcut::new("u", "Undo"),
                Shortcut::new("Ctrl+y", "Redo"),
                Shortcut::new("/", "Search"),
                Shortcut::new("Ctrl+f", "Global Search"),
                Shortcut::new("y/c", "Copy"),
                Shortcut::new("b", "Branch Filter"),
            ];
            if tab_name == "Archive" {
                shortcuts.push(Shortcut::new("r", "Restore"));
            }
            if tab_name == "Settings" {
                shortcuts.push(Shortcut::new("Space", "Toggle"));
            }
            shortcuts
        }
        "Move" => vec![
            Shortcut::new("j/k", "Move"),
            Shortcut::new("Esc/m/Ent", "Back"),
        ],
        "Editor" => {
            if has_suggestions {
                vec![
                    Shortcut::new("Up/Down", "Select"),
                    Shortcut::new("Enter", "Complete"),
                    Shortcut::new("Esc", "Close"),
                ]
            } else {
                vec![
                    Shortcut::new("Ctrl+s", "Save"),
                    Shortcut::new("Ctrl+g", "Save & Stage"),
                    Shortcut::new("Esc", "Cancel"),
                ]
            }
        }
        "Search" | "Global Search" => vec![
            Shortcut::new("Enter", "Confirm"),
            Shortcut::new("Esc", "Cancel"),
        ],
        "Confirm Discard" => vec![
            Shortcut::new("y", "Discard"),
            Shortcut::new("n", "Cancel"),
        ],
        _ => vec![],
    }
}
