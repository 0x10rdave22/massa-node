use std::fmt::Formatter;
use std::ops::Bound::{Excluded, Included};

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::error::{context, ContextError, ParseError};
use nom::multi::{count, length_count};
use nom::sequence::{preceded, tuple};
use nom::IResult;
use nom::Parser;
use serde::{Deserialize, Serialize};

use crate::block_id::BlockId;
use crate::endorsement::{
    Endorsement, EndorsementDeserializerLW, EndorsementId, EndorsementSerializer,
    EndorsementSerializerLW, SecureShareEndorsement,
};
use crate::secure_share::{
    SecureShare, SecureShareContent, SecureShareDeserializer, SecureShareSerializer,
};
use crate::slot::{Slot, SlotDeserializer, SlotSerializer};

use massa_hash::{Hash, HashDeserializer};
use massa_serialization::{
    Deserializer, SerializeError, Serializer, U32VarIntDeserializer, U32VarIntSerializer,
};

/// block header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// block version current
    pub block_version_current: u32,
    /// block version next
    pub block_version_next: u32,
    /// slot
    pub slot: Slot,
    /// parents
    pub parents: Vec<BlockId>,
    /// all operations hash
    pub operation_merkle_root: Hash,
    /// endorsements
    pub endorsements: Vec<SecureShareEndorsement>,
}

// NOTE: TODO
// impl Signable<BlockId> for BlockHeader {
//     fn get_signature_message(&self) -> Result<Hash, ModelsError> {
//         let hash = self.compute_hash()?;
//         let mut res = [0u8; SLOT_KEY_SIZE + BLOCK_ID_SIZE_BYTES];
//         res[..SLOT_KEY_SIZE].copy_from_slice(&self.slot.to_bytes_key());
//         res[SLOT_KEY_SIZE..].copy_from_slice(hash.to_bytes());
//         // rehash for safety
//         Ok(Hash::compute_from(&res))
//     }
// }

/// BlockHeader wrapped up alongside verification data
pub type SecuredHeader = SecureShare<BlockHeader, BlockId>;

impl SecuredHeader {
    /// gets the header fitness
    pub fn get_fitness(&self) -> u64 {
        (self.content.endorsements.len() as u64) + 1
    }
}

impl SecureShareContent for BlockHeader {}

/// Serializer for `BlockHeader`
pub struct BlockHeaderSerializer {
    slot_serializer: SlotSerializer,
    endorsement_serializer: SecureShareSerializer,
    endorsement_content_serializer: EndorsementSerializerLW,
    u32_serializer: U32VarIntSerializer,
}

impl BlockHeaderSerializer {
    /// Creates a new `BlockHeaderSerializer`
    pub fn new() -> Self {
        Self {
            slot_serializer: SlotSerializer::new(),
            endorsement_serializer: SecureShareSerializer::new(),
            u32_serializer: U32VarIntSerializer::new(),
            endorsement_content_serializer: EndorsementSerializerLW::new(),
        }
    }
}

impl Default for BlockHeaderSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer<BlockHeader> for BlockHeaderSerializer {
    /// ## Example:
    /// ```rust
    /// use massa_models::{block_id::BlockId, block_header::BlockHeader, block_header::BlockHeaderSerializer};
    /// use massa_models::endorsement::{Endorsement, EndorsementSerializer};
    /// use massa_models::secure_share::SecureShareContent;
    /// use massa_models::{config::THREAD_COUNT, slot::Slot};
    /// use massa_hash::Hash;
    /// use massa_signature::KeyPair;
    /// use massa_serialization::Serializer;
    ///
    /// let keypair = KeyPair::generate();
    /// let parents = (0..THREAD_COUNT)
    ///   .map(|i| BlockId(Hash::compute_from(&[i])))
    ///   .collect();
    /// let header = BlockHeader {
    ///   block_version_current: 0,block_version_next: 0,slot: Slot::new(1, 1),
    ///   parents,
    ///   operation_merkle_root: Hash::compute_from("mno".as_bytes()),
    ///   endorsements: vec![
    ///     Endorsement::new_verifiable(
    ///        Endorsement {
    ///          slot: Slot::new(1, 1),
    ///          index: 1,
    ///          endorsed_block: BlockId(Hash::compute_from("blk1".as_bytes())),
    ///        },
    ///     EndorsementSerializer::new(),
    ///     &keypair,
    ///     )
    ///     .unwrap(),
    ///     Endorsement::new_verifiable(
    ///       Endorsement {
    ///         slot: Slot::new(4, 0),
    ///         index: 3,
    ///         endorsed_block: BlockId(Hash::compute_from("blk2".as_bytes())),
    ///       },
    ///     EndorsementSerializer::new(),
    ///     &keypair,
    ///     )
    ///     .unwrap(),
    ///    ],
    /// };
    /// let mut buffer = vec![];
    /// BlockHeaderSerializer::new().serialize(&header, &mut buffer).unwrap();
    /// ```
    fn serialize(&self, value: &BlockHeader, buffer: &mut Vec<u8>) -> Result<(), SerializeError> {
        self.u32_serializer
            .serialize(&value.block_version_current, buffer)?;
        self.u32_serializer
            .serialize(&value.block_version_next, buffer)?;

        self.slot_serializer.serialize(&value.slot, buffer)?;
        // parents (note: there should be none if slot period=0)
        if value.parents.is_empty() {
            buffer.push(0);
        } else {
            buffer.push(1);
        }
        for parent_h in value.parents.iter() {
            buffer.extend(parent_h.0.to_bytes());
        }

        // operations merkle root
        buffer.extend(value.operation_merkle_root.to_bytes());

        self.u32_serializer.serialize(
            &value.endorsements.len().try_into().map_err(|err| {
                SerializeError::GeneralError(format!("too many endorsements: {}", err))
            })?,
            buffer,
        )?;
        for endorsement in value.endorsements.iter() {
            self.endorsement_serializer.serialize_with(
                &self.endorsement_content_serializer,
                endorsement,
                buffer,
            )?;
        }
        Ok(())
    }
}

/// Deserializer for `BlockHeader`
pub struct BlockHeaderDeserializer {
    slot_deserializer: SlotDeserializer,
    endorsement_serializer: EndorsementSerializer,
    length_endorsements_deserializer: U32VarIntDeserializer,
    hash_deserializer: HashDeserializer,
    thread_count: u8,
    endorsement_count: u32,
}

impl BlockHeaderDeserializer {
    /// Creates a new `BlockHeaderDeserializerLW`
    pub const fn new(thread_count: u8, endorsement_count: u32) -> Self {
        Self {
            slot_deserializer: SlotDeserializer::new(
                (Included(0), Included(u64::MAX)),
                (Included(0), Excluded(thread_count)),
            ),
            endorsement_serializer: EndorsementSerializer::new(),
            length_endorsements_deserializer: U32VarIntDeserializer::new(
                Included(0),
                Included(endorsement_count),
            ),
            hash_deserializer: HashDeserializer::new(),
            thread_count,
            endorsement_count,
        }
    }
}

impl Deserializer<BlockHeader> for BlockHeaderDeserializer {
    /// ## Example:
    /// ```rust
    /// use massa_models::block_id::BlockId;
    /// use massa_models::block_header::{BlockHeader, BlockHeaderDeserializer, BlockHeaderSerializer};
    /// use massa_models::{config::THREAD_COUNT, slot::Slot, secure_share::SecureShareContent};
    /// use massa_models::endorsement::{Endorsement, EndorsementSerializerLW};
    /// use massa_hash::Hash;
    /// use massa_signature::KeyPair;
    /// use massa_serialization::{Serializer, Deserializer, DeserializeError};
    ///
    /// let keypair = KeyPair::generate();
    /// let parents = (0..THREAD_COUNT)
    ///   .map(|i| BlockId(Hash::compute_from(&[i])))
    ///   .collect();
    /// let header = BlockHeader {
    ///   block_version_current: 0,block_version_next: 0,slot: Slot::new(1, 1),
    ///   parents,
    ///   operation_merkle_root: Hash::compute_from("mno".as_bytes()),
    ///   endorsements: vec![
    ///     Endorsement::new_verifiable(
    ///        Endorsement {
    ///          slot: Slot::new(1, 1),
    ///          index: 1,
    ///          endorsed_block: BlockId(Hash::compute_from("blk1".as_bytes())),
    ///        },
    ///     EndorsementSerializerLW::new(),
    ///     &keypair,
    ///     )
    ///     .unwrap(),
    ///     Endorsement::new_verifiable(
    ///       Endorsement {
    ///         slot: Slot::new(4, 0),
    ///         index: 3,
    ///         endorsed_block: BlockId(Hash::compute_from("blk2".as_bytes())),
    ///       },
    ///     EndorsementSerializerLW::new(),
    ///     &keypair,
    ///     )
    ///     .unwrap(),
    ///    ],
    /// };
    /// let mut buffer = vec![];
    /// BlockHeaderSerializer::new().serialize(&header, &mut buffer).unwrap();
    /// let (rest, deserialized_header) = BlockHeaderDeserializer::new(32, 9).deserialize::<DeserializeError>(&buffer).unwrap();
    /// assert_eq!(rest.len(), 0);
    /// let mut buffer2 = Vec::new();
    /// BlockHeaderSerializer::new().serialize(&deserialized_header, &mut buffer2).unwrap();
    /// assert_eq!(buffer, buffer2);
    /// ```
    fn deserialize<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        &self,
        buffer: &'a [u8],
    ) -> IResult<&'a [u8], BlockHeader, E> {
        let (rest, (version_cur, version_next, slot, parents, operation_merkle_root)): (
            &[u8],
            (u32, u32, Slot, Vec<BlockId>, Hash),
        ) = context(
            "Failed BlockHeader deserialization",
            tuple((
                context("Failed current version deserialization", |input| {
                    self.length_endorsements_deserializer.deserialize(input)
                }),
                context("Failed next version deserialization", |input| {
                    self.length_endorsements_deserializer.deserialize(input)
                }),
                context("Failed slot deserialization", |input| {
                    self.slot_deserializer.deserialize(input)
                }),
                context(
                    "Failed parents deserialization",
                    alt((
                        preceded(tag(&[0]), |input| Ok((input, Vec::new()))),
                        preceded(
                            tag(&[1]),
                            count(
                                context("Failed block_id deserialization", |input| {
                                    self.hash_deserializer
                                        .deserialize(input)
                                        .map(|(rest, hash)| (rest, BlockId(hash)))
                                }),
                                self.thread_count as usize,
                            ),
                        ),
                    )),
                ),
                context("Failed operation_merkle_root", |input| {
                    self.hash_deserializer.deserialize(input)
                }),
            )),
        )
        .parse(buffer)?;

        if parents.is_empty() {
            return Ok((
                &rest[1..], // Because there is 0 endorsements, we have a remaining 0 in rest and we don't need it
                BlockHeader {
                    block_version_current: version_cur,
                    block_version_next: version_next,
                    slot,
                    parents,
                    operation_merkle_root,
                    endorsements: Vec::new(),
                },
            ));
        }
        // Now deser the endorsements (which were: lw serialized)
        let endorsement_deserializer =
            SecureShareDeserializer::new(EndorsementDeserializerLW::new(
                self.endorsement_count,
                slot,
                parents[slot.thread as usize],
            ));

        let (rest, endorsements) = context(
            "Failed endorsements deserialization",
            length_count::<&[u8], SecureShare<Endorsement, EndorsementId>, u32, E, _, _>(
                context("Failed length deserialization", |input| {
                    self.length_endorsements_deserializer.deserialize(input)
                }),
                context("Failed endorsement deserialization", |input| {
                    endorsement_deserializer.deserialize_with(&self.endorsement_serializer, input)
                }),
            ),
        )
        .parse(rest)?;

        Ok((
            rest,
            BlockHeader {
                block_version_current: version_cur,
                block_version_next: version_next,
                slot,
                parents,
                operation_merkle_root,
                endorsements,
            },
        ))
    }
}

impl std::fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\t(period: {}, thread: {})",
            self.slot.period, self.slot.thread,
        )?;
        writeln!(f, "\tMerkle root: {}", self.operation_merkle_root,)?;
        writeln!(f, "\tParents: ")?;
        for id in self.parents.iter() {
            let str_id = id.to_string();
            writeln!(f, "\t\t{}", str_id)?;
        }
        if self.parents.is_empty() {
            writeln!(f, "No parents found: This is a genesis header")?;
        }
        writeln!(f, "\tEndorsements:")?;
        for ed in self.endorsements.iter() {
            writeln!(f, "\t\t-----")?;
            writeln!(f, "\t\tId: {}", ed.id)?;
            writeln!(f, "\t\tIndex: {}", ed.content.index)?;
            writeln!(f, "\t\tEndorsed slot: {}", ed.content.slot)?;
            writeln!(
                f,
                "\t\tEndorser's public key: {}",
                ed.content_creator_pub_key
            )?;
            writeln!(f, "\t\tEndorsed block: {}", ed.content.endorsed_block)?;
            writeln!(f, "\t\tSignature: {}", ed.signature)?;
        }
        if self.endorsements.is_empty() {
            writeln!(f, "\tNo endorsements found")?;
        }
        Ok(())
    }
}
