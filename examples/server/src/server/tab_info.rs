use std::sync::{Arc, Mutex};

use pkmc_defs::{packet, text_component::TextComponent};
use pkmc_util::{Color, WeakList, connection::ConnectionSender};

use crate::player::PlayerError;

#[derive(Debug)]
pub struct ServerTabInfo {
    sys: sysinfo::System,
    pid: sysinfo::Pid,
    viewers: WeakList<Mutex<ConnectionSender>>,
}

impl ServerTabInfo {
    pub fn new() -> Self {
        Self {
            sys: sysinfo::System::new_with_specifics(
                sysinfo::RefreshKind::nothing().with_processes(
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                ),
            ),
            pid: sysinfo::get_current_pid().unwrap(),
            viewers: WeakList::new(),
        }
    }

    pub fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<ConnectionSender>> {
        self.viewers.push(Mutex::new(connection))
    }

    pub fn update(&mut self) -> Result<(), PlayerError> {
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), true);
        let process = self.sys.process(self.pid).unwrap();
        self.viewers.iter().try_for_each(|viewer| {
            viewer.send(&packet::play::SetTabListHeaderAndFooter {
                header: None,
                footer: Some(
                    TextComponent::empty()
                        .with_child(|child| child.with_content("CPU: ").with_color(Color::GOLD))
                        .with_child(|child| {
                            child
                                .with_content(format!("{:.1}%", process.cpu_usage()))
                                .with_color(Color::YELLOW)
                        })
                        .with_child(|child| child.with_content(" - ").with_color(Color::DARK_GRAY))
                        .with_child(|child| {
                            child.with_content("MEM: ").with_color(Color::DARK_PURPLE)
                        })
                        .with_child(|child| {
                            child
                                .with_content(format!("{}MiB", process.memory() / 1048576))
                                .with_color(Color::LIGHT_PURPLE)
                        }),
                ),
            })
        })?;
        Ok(())
    }
}
