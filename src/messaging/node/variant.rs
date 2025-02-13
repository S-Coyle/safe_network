// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::{
    agreement::{DkgFailureSig, DkgFailureSigSet, DkgKey, Proposal, SectionSigned},
    join::{JoinRequest, JoinResponse},
    join_as_relocated::{JoinAsRelocatedRequest, JoinAsRelocatedResponse},
    network::Network,
    relocation::{RelocateDetails, RelocatePromise},
    section::{ElderCandidates, Section},
    signed::SigShare,
    RoutingMsg,
};
use crate::messaging::{DstInfo, SectionAuthorityProvider};
use bls::PublicKey as BlsPublicKey;
use bls_dkg::key_gen::message::Message as DkgMessage;
use hex_fmt::HexFmt;
use itertools::Itertools;
use secured_linked_list::SecuredLinkedList;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt::{self, Debug, Formatter},
};
use xor_name::XorName;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
/// Message variant
pub enum Variant {
    /// Inform other sections about our section or vice-versa.
    SectionKnowledge {
        /// `SectionAuthorityProvider` and `SecuredLinkedList` of the sender's section, with the proof chain.
        src_info: (SectionSigned<SectionAuthorityProvider>, SecuredLinkedList),
        /// Message
        msg: Option<Box<RoutingMsg>>,
    },
    /// User-facing message
    #[serde(with = "serde_bytes")]
    UserMessage(Vec<u8>),
    /// Message sent to all members to update them about the state of our section.
    Sync {
        /// Information about our section.
        section: Section,
        /// Information about the rest of the network that we know of.
        network: Network,
    },
    /// Send from a section to the node to be immediately relocated.
    Relocate(RelocateDetails),
    /// Send:
    /// - from a section to a current elder to be relocated after they are demoted.
    /// - from the node to be relocated back to its section after it was demoted.
    RelocatePromise(RelocatePromise),
    /// Sent from a bootstrapping peer to the section requesting to join as a new member
    JoinRequest(Box<JoinRequest>),
    /// Response to a `JoinRequest`
    JoinResponse(Box<JoinResponse>),
    /// Sent from a peer to the section requesting to join as relocated from another section
    JoinAsRelocatedRequest(Box<JoinAsRelocatedRequest>),
    /// Response to a `JoinAsRelocatedRequest`
    JoinAsRelocatedResponse(Box<JoinAsRelocatedResponse>),
    /// Sent from a node that can't establish the trust of the contained message to its original
    /// source in order for them to provide new proof that the node would trust.
    BouncedUntrustedMessage {
        /// Routing message
        msg: Box<RoutingMsg>,
        /// Destination info
        dst_info: DstInfo,
    },
    /// Sent to the new elder candidates to start the DKG process.
    DkgStart {
        /// The identifier of the DKG session to start.
        dkg_key: DkgKey,
        /// The DKG particpants.
        elder_candidates: ElderCandidates,
    },
    /// Message exchanged for DKG process.
    DkgMessage {
        /// The identifier of the DKG session this message is for.
        dkg_key: DkgKey,
        /// The DKG message.
        message: DkgMessage,
    },
    /// Broadcast to the other DKG participants when a DKG failure is observed.
    DkgFailureObservation {
        /// The DKG key
        dkg_key: DkgKey,
        /// Signature over the failure
        sig: DkgFailureSig,
        /// Nodes that failed to participate
        failed_participants: BTreeSet<XorName>,
    },
    /// Sent to the current elders by the DKG participants when at least majority of them observe
    /// a DKG failure.
    DkgFailureAgreement(DkgFailureSigSet),
    /// Message containing a single `Proposal` to be aggregated in the proposal aggregator.
    Propose {
        /// The content of the proposal
        content: Proposal,
        /// BLS signature share
        sig_share: SigShare,
    },
    /// Message that notifies a section to test
    /// the connectivity to a node
    StartConnectivityTest(XorName),
    /// Message sent by a node to indicate it received a message from a node which was ahead in knowledge.
    /// A reply is expected with a `SectionKnowledge` message.
    SectionKnowledgeQuery {
        /// Last known key by our node, used to get any newer keys
        last_known_key: Option<BlsPublicKey>,
        /// Routing message
        msg: Box<RoutingMsg>,
    },
}

impl Debug for Variant {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::SectionKnowledge { .. } => f.debug_struct("SectionKnowledge").finish(),
            Self::UserMessage(payload) => write!(f, "UserMessage({:10})", HexFmt(payload)),
            Self::Sync { section, network } => f
                .debug_struct("Sync")
                .field("section_auth", &section.section_auth.value)
                .field("section_key", section.chain.last_key())
                .field(
                    "other_prefixes",
                    &format_args!(
                        "({:b})",
                        network
                            .sections
                            .iter()
                            .map(|info| &info.section_auth.value.prefix)
                            .format(", ")
                    ),
                )
                .finish(),
            Self::Relocate(payload) => write!(f, "Relocate({:?})", payload),
            Self::StartConnectivityTest(name) => write!(f, "StartConnectivityTest({:?})", name),
            Self::RelocatePromise(payload) => write!(f, "RelocatePromise({:?})", payload),
            Self::JoinRequest(payload) => write!(f, "JoinRequest({:?})", payload),
            Self::JoinResponse(response) => write!(f, "JoinResponse({:?})", response),
            Self::JoinAsRelocatedRequest(payload) => {
                write!(f, "JoinAsRelocatedRequest({:?})", payload)
            }
            Self::JoinAsRelocatedResponse(response) => {
                write!(f, "JoinAsRelocatedResponse({:?})", response)
            }
            Self::BouncedUntrustedMessage { msg, dst_info } => f
                .debug_struct("BouncedUntrustedMessage")
                .field("message", msg)
                .field("dst_info", dst_info)
                .finish(),
            Self::DkgStart {
                dkg_key,
                elder_candidates,
            } => f
                .debug_struct("DkgStart")
                .field("dkg_key", dkg_key)
                .field("elder_candidates", elder_candidates)
                .finish(),
            Self::DkgMessage { dkg_key, message } => f
                .debug_struct("DkgMessage")
                .field("dkg_key", &dkg_key)
                .field("message", message)
                .finish(),
            Self::DkgFailureObservation {
                dkg_key,
                sig,
                failed_participants,
            } => f
                .debug_struct("DkgFailureObservation")
                .field("dkg_key", dkg_key)
                .field("sig", sig)
                .field("failed_participants", failed_participants)
                .finish(),
            Self::DkgFailureAgreement(proofs) => {
                f.debug_tuple("DkgFailureAgreement").field(proofs).finish()
            }
            Self::Propose { content, sig_share } => f
                .debug_struct("Propose")
                .field("content", content)
                .field("sig_share", sig_share)
                .finish(),
            Self::SectionKnowledgeQuery { .. } => write!(f, "SectionKnowledgeQuery"),
        }
    }
}
