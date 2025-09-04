use std::sync::Arc;
use sui_types::{
    base_types::SuiAddress,
    crypto::{PublicKey, Signature, SuiSignature},
    signature_verification::{verify_sender_signed_data_message_signatures, VerifiedDigestCache},
    transaction::Transaction,
};

#[derive(Clone, Copy, Debug)]
pub enum AuthMode {
    Enabled,
    Disabled,
}

pub struct AuthExtension {
    mode: AuthMode,
}

impl AuthExtension {
    pub fn new() -> Self {
        Self {
            mode: AuthMode::Enabled,
        }
    }

    pub fn verify_transaction(&self, transaction: &Transaction, epoch: u64) -> anyhow::Result<()> {
        if self.is_disabled() {
            return Ok(());
        }

        verify_sender_signed_data_message_signatures(
            transaction,
            epoch,
            &Default::default(),
            Arc::new(VerifiedDigestCache::new_empty()),
            None,
        )?;

        Ok(())
    }

    pub fn verify_object_ownership(
        &self,
        object_owner: SuiAddress,
        signatures: &[Signature],
    ) -> anyhow::Result<()> {
        if self.is_disabled() {
            return Ok(());
        }

        let authorized = signatures.iter().any(|signature| {
            PublicKey::try_from_bytes(signature.scheme(), signature.public_key_bytes())
                .ok()
                .map(|pk| SuiAddress::from(&pk))
                .map(|addr| addr == object_owner)
                .unwrap_or(false)
        });

        if !authorized {
            anyhow::bail!(
                "Object owned by {} accessed without owner signature",
                object_owner
            )
        }

        Ok(())
    }

    pub fn set_mode(&mut self, mode: AuthMode) -> AuthMode {
        let prev = self.mode;
        self.mode = mode;

        prev
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self.mode, AuthMode::Disabled)
    }
}
