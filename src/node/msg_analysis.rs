// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use routing::Node as Routing;
use safe_nd::{
    Address, Cmd, DataCmd, Duty, ElderDuty, Message, MsgEnvelope, MsgSender, Query, XorName,
};
use std::{cell::RefCell, rc::Rc};

#[allow(clippy::large_enum_variant)]
pub(crate) enum NodeDuties {
    Infant,
    Adult,
    Elder,
}

/// Currently, this is only evaluating
/// remote msgs from the network, i.e.
/// it is not evaluating msgs sent
/// directly from the client.
pub(crate) struct InboundMsgAnalysis {
    routing: Rc<RefCell<Routing>>,
}

pub(crate) enum InboundMsg {
    Accumulate(MsgEnvelope),
    ForwardToNetwork(MsgEnvelope),
    RunAtGateway(MsgEnvelope),
    RunAtPayment(MsgEnvelope),
    RunAtMetadata(MsgEnvelope),
    RunAtAdult(MsgEnvelope),
    SendToClient(MsgEnvelope),
    RunAtRewards(MsgEnvelope),
    RunAtTransfers(MsgEnvelope),
    Unknown,
}

impl InboundMsgAnalysis {
    pub fn new(routing: Rc<RefCell<Routing>>) -> Self {
        Self { routing }
    }

    pub fn is_dst_for(&self, msg: &MsgEnvelope) -> bool {
        self.self_is_handler_for(&msg.destination().xorname())
    }

    /// Currently, this is only evaluating
    /// remote msgs from the network, i.e.
    /// it is not evaluating msgs sent
    /// directly from the client.
    pub fn evaluate(&self, msg: &MsgEnvelope) -> InboundMsg {
        if self.should_accumulate(msg) {
            InboundMsg::Accumulate(msg.clone())
        } else if self.should_forward_to_network(msg) {
            // Any type of msg that is not process locally.
            InboundMsg::ForwardToNetwork(msg.clone())
        } else if self.should_push_to_client(msg) {
            // From network to client!
            InboundMsg::SendToClient(msg.clone())
        } else if self.should_run_at_gateway_auth(msg) {
            // Client auth cmd finalisation (Temporarily handled here, will be at app layer (Authenticator)).
            // The auth cmd has been agreed by the Gateway section.
            // (All other client msgs are handled when received from client).
            InboundMsg::RunAtGateway(msg.clone())
        } else if self.should_run_at_data_payment(msg) {
            // Incoming msg from `Gateway`!
            InboundMsg::RunAtPayment(msg.clone()) // Payment Elders should just execute and send onwards.
        } else if self.should_run_at_metadata_write(msg) {
            // Accumulated msg from `Payment`!
            InboundMsg::RunAtMetadata(msg.clone())
        } else if self.should_run_at_adult(msg) {
            // Accumulated msg from `Metadata`!
            InboundMsg::RunAtAdult(msg.clone())
        } else if self.should_run_at_rewards(msg) {
            InboundMsg::RunAtRewards(msg.clone())
        } else if self.should_run_at_transfers(msg) {
            InboundMsg::RunAtTransfers(msg.clone())
        } else {
            InboundMsg::Unknown
        }
    }

    fn should_accumulate(&self, msg: &MsgEnvelope) -> bool {
        // Incoming msg from `Payment`!
        self.should_accumulate_for_metadata_write(msg) // Metadata Elders accumulate the msgs from Payment Elders.
        // Incoming msg from `Metadata`!
        || self.should_accumulate_for_adult(msg) // Adults accumulate the msgs from Metadata Elders.
    }

    fn should_forward_to_network(&self, msg: &MsgEnvelope) -> bool {
        use Address::*;
        let destined_for_network = || match msg.destination() {
            Client(address) => !self.self_is_handler_for(&address),
            Node(address) => routing::XorName(address.0) != *self.routing.borrow().id().name(),
            Section(address) => !self.self_is_handler_for(&address),
        };
        let from_client = || match msg.most_recent_sender() {
            MsgSender::Client { .. } => true,
            _ => false,
        };
        let is_auth_cmd = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Auth { .. },
                ..
            } => true,
            _ => false,
        };

        destined_for_network() || (from_client() && !is_auth_cmd())
    }

    // todo: eval all msg types!
    fn should_run_at_gateway_auth(&self, msg: &MsgEnvelope) -> bool {
        let from_client = || match msg.origin {
            MsgSender::Client { .. } => true,
            _ => false,
        };
        let agreed_by_gateway_section = || match msg.most_recent_sender() {
            MsgSender::Section {
                duty: Duty::Elder(ElderDuty::Gateway),
                ..
            } => true,
            _ => false,
        };
        let is_auth_cmd = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Auth { .. },
                ..
            } => true,
            _ => false,
        };

        from_client()
            && agreed_by_gateway_section()
            && is_auth_cmd()
            && self.is_dst_for(msg)
            && self.is_elder()
    }

    /// We do not accumulate these request, they are executed
    /// at once (i.e. payment carried out) and sent on to
    /// Metadata section. (They however, will accumulate those msgs.)
    /// The reason for this is that the payment request is already signed
    /// by the client and validated by its replicas,
    /// so there is no reason to accumulate it here.
    fn should_run_at_data_payment(&self, msg: &MsgEnvelope) -> bool {
        let from_gateway_single_elder = || match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Gateway),
                ..
            } => true,
            _ => false,
        };
        let is_data_msg = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            }
            | Message::Query {
                query: Query::Data(_),
                ..
            } => true,
            _ => false,
        };

        is_data_msg() && from_gateway_single_elder() && self.is_dst_for(msg) && self.is_elder()
    }

    /// The individual Payment Elder nodes send their msgs
    /// to Metadata section, where it is accumulated.
    fn should_accumulate_for_metadata_write(&self, msg: &MsgEnvelope) -> bool {
        let from_single_payment_elder = || match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Payment),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            } => true,
            _ => false,
        };

        is_data_cmd() && from_single_payment_elder() && self.is_dst_for(msg) && self.is_elder()
    }

    /// After the data write sent from Payment Elders has been
    /// accumulated (can be seen since the sender is `Section`),
    /// it is time to actually carry out the write operation.
    fn should_run_at_metadata_write(&self, msg: &MsgEnvelope) -> bool {
        let from_payment_section = || match msg.most_recent_sender() {
            MsgSender::Section {
                duty: Duty::Elder(ElderDuty::Payment),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            } => true,
            _ => false,
        };

        is_data_cmd() && from_payment_section() && self.is_dst_for(msg) && self.is_elder()
    }

    /// Adults accumulate the write requests from Elders.
    fn should_accumulate_for_adult(&self, msg: &MsgEnvelope) -> bool {
        let from_single_metadata_elder = || match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Metadata),
                ..
            } => true,
            _ => false,
        };
        let is_chunk_cmd = || match msg.message {
            Message::Cmd {
                cmd:
                    Cmd::Data {
                        cmd: DataCmd::Blob(_),
                        ..
                    },
                ..
            } => true,
            _ => false,
        };

        is_chunk_cmd() && from_single_metadata_elder() && self.is_dst_for(msg) && self.is_adult()
    }

    /// When the write requests from Elders has been accumulated
    /// at an Adult, it is time to carry out the write operation.
    fn should_run_at_adult(&self, msg: &MsgEnvelope) -> bool {
        let from_metadata_section = || match msg.most_recent_sender() {
            MsgSender::Section {
                duty: Duty::Elder(ElderDuty::Metadata),
                ..
            } => true,
            _ => false,
        };
        let is_chunk_cmd = || match msg.message {
            Message::Cmd {
                cmd:
                    Cmd::Data {
                        cmd: DataCmd::Blob(_),
                        ..
                    },
                ..
            } => true,
            _ => false,
        };

        is_chunk_cmd() && from_metadata_section() && self.is_dst_for(msg) && self.is_adult()
    }

    fn should_run_at_rewards(&self, _msg: &MsgEnvelope) -> bool {
        false //unimplemented
    }

    fn should_run_at_transfers(&self, msg: &MsgEnvelope) -> bool {
        let from_single_gateway_elder = || match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Gateway),
                ..
            } => true,
            _ => false,
        };
        let is_transfer = || match msg.message {
            Message::Cmd {
                cmd: Cmd::Transfer(_),
                ..
            } => true,
            _ => false,
        };

        is_transfer() && from_single_gateway_elder() && self.is_dst_for(msg) && self.is_elder()
    }

    fn should_push_to_client(&self, msg: &MsgEnvelope) -> bool {
        match msg.destination() {
            Address::Client(xorname) => self.self_is_handler_for(&xorname),
            _ => false,
        }
    }

    fn self_is_handler_for(&self, address: &XorName) -> bool {
        let xorname = routing::XorName(address.0);
        match self.routing.borrow().matches_our_prefix(&xorname) {
            Ok(result) => result,
            _ => false,
        }
    }

    fn is_elder(&self) -> bool {
        if let NodeDuties::Elder = self.our_duties() {
            true
        } else {
            false
        }
    }

    fn is_adult(&self) -> bool {
        if let NodeDuties::Adult = self.our_duties() {
            true
        } else {
            false
        }
    }

    fn our_duties(&self) -> NodeDuties {
        if self.routing.borrow().is_elder() {
            NodeDuties::Elder
        } else if self
            .routing
            .borrow()
            .our_adults()
            .map(|c| c.name())
            .any(|x| x == self.routing.borrow().name())
        {
            NodeDuties::Adult
        } else {
            NodeDuties::Infant
        }
    }
}
