use std::{sync::Arc, time::Duration};

use tokio::{sync::RwLock, time::Instant};

use crate::{
    invitation::InvitationToken,
    party::{PartyId, Role},
    store::MockStore,
    user::UserId,
};

pub struct DebugApp {
    state: Arc<RwLock<MockStore>>,
}

struct UserView {
    id: UserId,
    email: String,
    username: String,
}

struct PartyView {
    id: PartyId,
    creator: UserId,
    members: Vec<(UserId, Role)>,
}

struct InvitationView {
    token: InvitationToken,
    party_id: PartyId,
    expires_in: Duration,
}

impl DebugApp {
    pub fn new(store: Arc<RwLock<MockStore>>) -> Self {
        Self { state: store }
    }

    fn users_section(&self, ui: &mut egui::Ui, users: &[UserView]) {
        egui::CollapsingHeader::new(format!("Users ({})", users.len()))
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("users_grid").striped(true).show(ui, |ui| {
                    ui.label("ID");
                    ui.label("Email");
                    ui.label("Username");
                    ui.end_row();

                    for u in users {
                        ui.monospace(u.id.to_string());
                        ui.label(&u.email);
                        ui.label(&u.username);
                        ui.end_row();
                    }
                });
            });
    }

    fn parties_section(&self, ui: &mut egui::Ui, parties: &[PartyView]) {
        egui::CollapsingHeader::new(format!("Parties ({})", parties.len()))
            .default_open(false)
            .show(ui, |ui| {
                for party in parties {
                    egui::CollapsingHeader::new(format!(
                        "Party {} ({} members)",
                        party.id,
                        party.members.len()
                    ))
                    .show(ui, |ui| {
                        ui.label(format!("Creator: {}", party.creator));
                        ui.separator();

                        for (user_id, role) in &party.members {
                            ui.horizontal(|ui| {
                                ui.monospace(user_id.to_string());
                                ui.label(format!("{:?}", role));
                            });
                        }
                    });
                }
            });
    }

    fn invitations_section(&self, ui: &mut egui::Ui, invitations: &[InvitationView]) {
        egui::CollapsingHeader::new(format!("Invitation Tokens ({})", invitations.len()))
            .default_open(false)
            .show(ui, |ui| {
                for inv in invitations {
                    ui.horizontal(|ui| {
                        ui.monospace(inv.token.to_string());
                        ui.label(format!("Party: {}", inv.party_id));

                        if inv.expires_in.is_zero() {
                            ui.colored_label(egui::Color32::RED, "EXPIRED");
                        } else {
                            ui.label(format!("expires in {}s", inv.expires_in.as_secs()));
                        }
                    });
                }
            });
    }
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let snapshot = {
            let store = self.state.blocking_read();
            DebugSnapshot::from_store(&store)
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("FriendlyFire Server Debug");
            ui.separator();

            self.users_section(ui, &snapshot.users);
            self.parties_section(ui, &snapshot.parties);
            self.invitations_section(ui, &snapshot.invitations);
        });
    }
}

pub struct DebugSnapshot {
    users: Vec<UserView>,
    parties: Vec<PartyView>,
    invitations: Vec<InvitationView>,
}

impl DebugSnapshot {
    fn from_store(store: &MockStore) -> Self {
        let now = Instant::now();

        Self {
            users: store
                .users
                .values()
                .map(|u| UserView {
                    id: u.id,
                    email: u.email.clone(),
                    username: u.username.clone(),
                })
                .collect(),

            parties: store
                .parties
                .values()
                .map(|p| PartyView {
                    id: p.id,
                    creator: p.creator.id,
                    members: p.members.iter().map(|(id, r)| (*id, r.clone())).collect(),
                })
                .collect(),

            invitations: store
                .invitations
                .values()
                .map(|i| InvitationView {
                    token: i.token,
                    party_id: i.party_id,
                    expires_in: i.valid_until.saturating_duration_since(now),
                })
                .collect(),
        }
    }
}
