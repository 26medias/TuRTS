//! Host-linked factories. Portable package loading does not require this registry.
use crate::{controller::BotController, seed::derive_seed, types::BotActor};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};
use turts_mod_api::{BotControllerDef, BotDef, BotQuota, ContentId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryError {
    pub field_path: String,
    pub message: String,
}
impl FactoryError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field_path: field.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field_path, self.message)
    }
}
impl std::error::Error for FactoryError {}
pub struct ControllerInit<'a> {
    pub actor: &'a BotActor,
    pub profile: &'a BotDef,
    pub instance: &'a BotControllerDef,
    pub seed: u64,
}
/// Factories create fresh match-local state and perform no gameplay mutation.
pub trait ControllerFactory: Send + Sync {
    fn validate(&self, profile: &BotDef, instance: &BotControllerDef) -> Result<(), FactoryError>;
    fn create(&self, init: ControllerInit<'_>) -> Result<Box<dyn BotController>, FactoryError>;
}
pub struct ControllerInstance {
    pub key: String,
    pub quota: BotQuota,
    pub seed: u64,
    pub controller: Box<dyn BotController>,
}
#[derive(Default)]
pub struct ControllerRegistry {
    factories: BTreeMap<ContentId, Arc<dyn ControllerFactory>>,
}
impl ControllerRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(
        &mut self,
        id: ContentId,
        factory: Arc<dyn ControllerFactory>,
    ) -> Result<(), FactoryError> {
        if self.factories.contains_key(&id) {
            return Err(FactoryError::new(
                "factory",
                format!("factory `{id}` is already registered"),
            ));
        }
        self.factories.insert(id, factory);
        Ok(())
    }
    pub fn contains(&self, id: &ContentId) -> bool {
        self.factories.contains_key(id)
    }
    /// Validate the entire profile before creating any controller or claiming a host slot.
    pub fn activate(
        &self,
        actor: &BotActor,
        profile: &BotDef,
        seed: u64,
        limit: BotQuota,
    ) -> Result<Vec<ControllerInstance>, FactoryError> {
        if actor.profile.local != profile.id {
            return Err(FactoryError::new(
                "id",
                "profile identity does not match its catalog key",
            ));
        }
        let mut keys = BTreeSet::new();
        let mut sum = BotQuota::default();
        let mut prepared = Vec::new();
        for (index, instance) in profile.controllers.iter().enumerate() {
            let field = format!("controllers[{index}]");
            if instance.key.is_empty() || !keys.insert(instance.key.as_str()) {
                return Err(FactoryError::new(
                    format!("{field}.key"),
                    "controller keys must be nonempty and unique",
                ));
            }
            for (name, total, added, max) in [
                (
                    "queries",
                    &mut sum.queries,
                    instance.quota.queries,
                    limit.queries,
                ),
                (
                    "candidates",
                    &mut sum.candidates,
                    instance.quota.candidates,
                    limit.candidates,
                ),
                ("rows", &mut sum.rows, instance.quota.rows, limit.rows),
                (
                    "proposals",
                    &mut sum.proposals,
                    instance.quota.proposals,
                    limit.proposals,
                ),
            ] {
                *total = total
                    .checked_add(added)
                    .filter(|value| *value <= max)
                    .ok_or_else(|| {
                        FactoryError::new(
                            format!("{field}.quota.{name}"),
                            "controller quotas exceed the operator limit",
                        )
                    })?;
            }
            let id = ContentId::parse(&instance.factory)
                .map_err(|e| FactoryError::new(format!("{field}.factory"), e.to_string()))?;
            let factory = self.factories.get(&id).ok_or_else(|| {
                FactoryError::new(
                    format!("{field}.factory"),
                    format!("factory `{id}` is not available in this host"),
                )
            })?;
            factory
                .validate(profile, instance)
                .map_err(|e| FactoryError::new(format!("{field}.{}", e.field_path), e.message))?;
            prepared.push((instance, factory));
        }
        let mut active = Vec::new();
        for (instance, factory) in prepared {
            let sub_seed = derive_seed(seed, &actor.profile, actor.slot, &instance.key, None);
            let controller = factory.create(ControllerInit {
                actor,
                profile,
                instance,
                seed: sub_seed,
            })?;
            active.push(ControllerInstance {
                key: instance.key.clone(),
                quota: instance.quota,
                seed: sub_seed,
                controller,
            });
        }
        // Registration order is irrelevant. Quotas belong to authored identities.
        active.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(active)
    }
}
