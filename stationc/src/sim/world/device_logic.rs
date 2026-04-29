//! Logic field names shared by world simulator devices.

pub(super) const REFERENCE_ID: &str = "ReferenceId";
pub(super) const PREFAB_HASH: &str = "PrefabHash";
pub(super) const NAME_HASH: &str = "NameHash";
pub(super) const ON: &str = "On";
pub(super) const SETTING: &str = "Setting";

pub(super) fn is_read_only(field: &str) -> bool {
    matches!(field, REFERENCE_ID | PREFAB_HASH | NAME_HASH)
}
