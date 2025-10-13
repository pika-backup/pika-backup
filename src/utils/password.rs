use std::collections::HashMap;
use std::sync::RwLock;

use crate::config::Password;
use crate::prelude::*;

#[derive(Default)]
pub struct MemoryPasswordStore {
    passwords: Arc<RwLock<HashMap<ConfigId, Password>>>,
}

impl MemoryPasswordStore {
    pub fn set_password(&self, config: &crate::config::Backup, password: Password) {
        self.passwords
            .write()
            .unwrap()
            .insert(config.id.clone(), password);
    }

    pub fn load_password(&self, config: &crate::config::Backup) -> Option<Password> {
        self.passwords.read().unwrap().get(&config.id).cloned()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_password_store() {
        let store = Arc::new(MemoryPasswordStore::default());

        let config = crate::config::Backup::test_new_mock();
        let password_str = "testpw";
        let password = Password::new(password_str.to_string());

        store.set_password(&config, password.clone());
        assert_eq!(
            store
                .load_password(&config)
                .map(|pw| pw.as_bytes().to_vec()),
            Some(password_str.as_bytes().to_vec()),
        );
    }
}
