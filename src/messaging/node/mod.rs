// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod agreement;
mod join;
mod join_as_relocated;
mod network;
mod node_msg;
mod plain_message;
mod prefix_map;
mod relocation;
mod section;
mod signature_aggregator;
mod signed;
mod src_authority;
mod variant;

pub use agreement::{DkgFailureSig, DkgFailureSigSet, DkgKey, Proposal, SectionSigned};
pub use join::{JoinRejectionReason, JoinRequest, JoinResponse, ResourceProofResponse};
pub use join_as_relocated::{JoinAsRelocatedRequest, JoinAsRelocatedResponse};
pub use network::{Network, OtherSection};
pub use node_msg::{
    NodeCmd, NodeCmdError, NodeDataError, NodeDataQueryResponse, NodeEvent, NodeMsg, NodeQuery,
    NodeQueryResponse, NodeRewardQuery, NodeSystemCmd, NodeSystemQuery, NodeSystemQueryResponse,
    NodeTransferCmd, NodeTransferError, NodeTransferQuery, NodeTransferQueryResponse,
};
pub use plain_message::PlainMessage;
pub use prefix_map::PrefixMap;
pub use relocation::{RelocateDetails, RelocatePayload, RelocatePromise, SignedRelocateDetails};
pub use section::{ElderCandidates, MembershipState, NodeState, Peer, Section, SectionPeers};
pub use signature_aggregator::{Error, SignatureAggregator};
pub use signed::{KeyedSig, SigShare};
pub use src_authority::SrcAuthority;
pub use variant::Variant;

use crate::messaging::{Aggregation, DstLocation, MessageId, MessageType, WireMsg};
use bls::PublicKey as BlsPublicKey;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Formatter};
use xor_name::XorName;

/// Routing message sent over the network.
#[derive(Clone, Eq, Serialize, Deserialize)]
pub struct RoutingMsg {
    /// Message ID.
    pub id: MessageId,
    /// Source authority.
    /// Messages do not need to sign this field as it is all verifiable (i.e. if the sig validates
    /// agains the public key and we know the pub key then we are good. If the proof is not recognised we
    /// ask for a longer chain that can be recognised). Therefore we don't need to sign this field.
    pub src: SrcAuthority,
    /// Destination location.
    pub dst: DstLocation,
    /// The aggregation scheme to be used.
    pub aggregation: Aggregation,
    /// The body of the message.
    pub variant: Variant,
    /// Section key of the sender.
    pub section_pk: BlsPublicKey,
}

impl RoutingMsg {
    /// Convenience function to deserialize a 'RoutingMsg' from bytes received over the wire.
    /// It returns an error if the bytes don't correspond to a node message.
    pub fn from(bytes: Bytes) -> crate::messaging::Result<Self> {
        let deserialized = WireMsg::deserialize(bytes)?;
        if let MessageType::Routing { msg, .. } = deserialized {
            Ok(msg)
        } else {
            Err(crate::messaging::Error::FailedToParse(
                "bytes as a node message".to_string(),
            ))
        }
    }

    /// serialize this RoutingMsg into bytes ready to be sent over the wire.
    pub fn serialize(
        &self,
        dst: XorName,
        dst_section_pk: BlsPublicKey,
    ) -> crate::messaging::Result<Bytes> {
        WireMsg::serialize_routing_msg(self, dst, dst_section_pk)
    }
}

impl PartialEq for RoutingMsg {
    fn eq(&self, other: &RoutingMsg) -> bool {
        self.src == other.src
            && self.dst == other.dst
            && self.id == other.id
            && self.variant == other.variant
            && self.section_pk == other.section_pk
    }
}

impl Debug for RoutingMsg {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("RoutingMsg")
            .field("id", &self.id)
            .field("src", &self.src)
            .field("dst", &self.dst)
            .field("variant", &self.variant)
            .finish()
    }
}
