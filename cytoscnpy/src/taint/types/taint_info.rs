use super::{TaintInfo, VulnType};

impl TaintInfo {
    /// Returns a copy marked safe for the specified vulnerability classes.
    #[must_use]
    pub fn with_sanitized_for(&self, vuln_types: &[VulnType]) -> Self {
        let mut info = self.clone();
        info.mark_sanitized_for(vuln_types);
        info
    }

    /// Marks this taint flow safe for the specified vulnerability classes.
    pub fn mark_sanitized_for(&mut self, vuln_types: &[VulnType]) {
        for vuln_type in vuln_types {
            if !self.sanitized_for.contains(vuln_type) {
                self.sanitized_for.push(vuln_type.clone());
            }
        }
    }

    /// Checks whether this flow is sanitized for a vulnerability class.
    #[must_use]
    pub fn is_sanitized_for(&self, vuln_type: &VulnType) -> bool {
        self.sanitized_for.contains(vuln_type)
    }
}
