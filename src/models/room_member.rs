// Copyright 2020 Damir Jelić
// Copyright 2020 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::User;
use crate::api::r0 as api;
use crate::events::collections::all::{Event, RoomEvent, StateEvent};
use crate::events::room::{
    aliases::AliasesEvent,
    canonical_alias::CanonicalAliasEvent,
    member::{MemberEvent, MemberEventContent, MembershipState},
    name::NameEvent,
    power_levels::PowerLevelsEvent,
};
use crate::events::EventResult;
use crate::identifiers::{RoomAliasId, UserId};
use crate::session::Session;

use js_int::{Int, UInt};
#[cfg(feature = "encryption")]
use tokio::sync::Mutex;

#[cfg(feature = "encryption")]
use crate::crypto::{OlmMachine, OneTimeKeys};
#[cfg(feature = "encryption")]
use ruma_client_api::r0::keys::{upload_keys::Response as KeysUploadResponse, DeviceKeys};

#[derive(Debug)]
/// A Matrix room member.
pub struct RoomMember {
    /// The unique mxid of the user.
    pub user_id: UserId,
    /// The unique id of the room.
    pub room_id: Option<String>,
    /// If the member is typing.
    pub typing: Option<bool>,
    /// The user data for this room member.
    pub user: User,
    /// The users power level.
    pub power_level: Option<Int>,
    /// The normalized power level of this `RoomMember` (0-100).
    pub power_level_norm: Option<Int>,
    /// The `MembershipState` of this `RoomMember`.
    pub membership: MembershipState,
    /// The human readable name of this room member.
    pub name: String,
    /// The events that created the state of this room member.
    pub events: Vec<Event>,
}

impl RoomMember {
    pub fn new(event: &MemberEvent) -> Self {
        let user = User::new(event);
        Self {
            room_id: event.room_id.as_ref().map(|id| id.to_string()),
            user_id: event.sender.clone(),
            typing: None,
            user,
            power_level: None,
            power_level_norm: None,
            membership: event.content.membership,
            name: event.state_key.clone(),
            events: vec![Event::RoomMember(event.clone())],
        }
    }

    pub fn update_member(&mut self, event: &MemberEvent) -> bool {
        let changed = self.membership == event.content.membership;
        self.membership = event.content.membership;
        changed
    }

    pub fn update_power(&mut self, event: &PowerLevelsEvent) -> bool {
        let mut max_power = event.content.users_default;
        for power in event.content.users.values() {
            max_power = *power.max(&max_power);
        }

        let mut changed = false;
        if let Some(user_power) = event.content.users.get(&self.user_id) {
            changed = self.power_level == Some(*user_power);
            self.power_level = Some(*user_power);
        } else {
            changed = self.power_level == Some(event.content.users_default);
            self.power_level = Some(event.content.users_default);
        }

        if max_power > Int::from(0) {
            self.power_level_norm = Some((self.power_level.unwrap() * Int::from(100)) / max_power);
        }

        changed
    }
}
