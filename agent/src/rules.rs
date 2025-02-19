/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

// Unused at this moment...

#[derive(Serialize,Deserialize,Clone,Debug)]
#[serde(transparent)]
pub struct HostRules<T>(Vec<HostRule<T>>);

#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct HostRule<T> {
    hosts: Option<HashSet<String>>,
    not_hosts: Option<HashSet<String>>,
    tags: Option<HashSet<Tag>>,
    not_tags: Option<HashSet<Tag>>,
    rule: T
}


impl<T> HostRules<T> {

    pub fn resolve(&self, host: &str, tags: &HashSet<Tag>) -> Option<&T> {

	for rule in &self.0 {
	    if rule.hosts.as_ref().map_or_else(|| true, |rule_hosts| rule_hosts.contains(host))
		&& rule.not_hosts.as_ref().map_or_else(|| true, |rule_hosts| !rule_hosts.contains(host))
		&& rule.tags.as_ref().map_or_else(|| true, |rule_tags| rule_tags.iter().all(
		    |rule_tag| tags.contains(rule_tag)))
		&& rule.not_tags.as_ref().map_or_else(|| true, |rule_tags| rule_tags.iter().all(
		    |rule_tag| !tags.contains(rule_tag))) {
		    return Some(&rule.rule)
		}
	}

	None

    }

}
