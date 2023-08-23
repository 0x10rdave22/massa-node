use std::collections::hash_map;

use massa_models::{
    address::Address,
    endorsement::{EndorsementId, SecureShareEndorsement},
    prehash::{PreHashMap, PreHashSet},
};

/// Container for all endorsements and different indexes.
/// Note: The structure can evolve and store more indexes.
#[derive(Default)]
pub struct EndorsementIndexes {
    /// Endorsements structure container
    endorsements: PreHashMap<EndorsementId, Box<SecureShareEndorsement>>,
    /// Structure mapping creators with the created endorsements
    index_by_creator: PreHashMap<Address, PreHashSet<EndorsementId>>,
}

impl EndorsementIndexes {
    /// Insert an endorsement and populate the indexes.
    /// Arguments:
    /// - endorsement: the endorsement to insert
    pub(crate) fn insert(&mut self, endorsement: SecureShareEndorsement) {
        if !self.endorsements.contains_key(&endorsement.id) {
            let endorsement = self
                .endorsements
                .entry(endorsement.id)
                .or_insert(Box::new(endorsement));
            // update creator index
            self.index_by_creator
                .entry(endorsement.content_creator_address)
                .or_default()
                .insert(endorsement.id);

            massa_metrics::set_endorsements_counter(self.endorsements.len());
        }
    }

    /// Remove a endorsement, remove from the indexes and made some clean-up in indexes if necessary.
    /// Arguments:
    /// * `endorsement_id`: the endorsement id to remove
    pub(crate) fn remove(
        &mut self,
        endorsement_id: &EndorsementId,
    ) -> Option<Box<SecureShareEndorsement>> {
        if let Some(e) = self.endorsements.remove(endorsement_id) {
            massa_metrics::set_endorsements_counter(self.endorsements.len());

            // update creator index
            if let hash_map::Entry::Occupied(mut occ) =
                self.index_by_creator.entry(e.content_creator_address)
            {
                occ.get_mut().remove(&e.id);
                if occ.get().is_empty() {
                    occ.remove();
                }
            }
            return Some(e);
        }
        None
    }

    /// Gets a reference to a stored endorsement, if any.
    pub fn get(&self, id: &EndorsementId) -> Option<&SecureShareEndorsement> {
        self.endorsements.get(id).map(|v| v.as_ref())
    }

    /// Checks whether an endorsement exists in global storage.
    pub fn contains(&self, id: &EndorsementId) -> bool {
        self.endorsements.contains_key(id)
    }

    /// Get endorsements created by an address
    /// Arguments:
    /// - address: the address to get the endorsements created by
    ///
    /// Returns:
    /// - optional reference to a set of endorsements created by that address
    pub fn get_endorsements_created_by(
        &self,
        address: &Address,
    ) -> Option<&PreHashSet<EndorsementId>> {
        self.index_by_creator.get(address)
    }
}
