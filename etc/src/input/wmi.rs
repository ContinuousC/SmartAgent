/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

/* WMI-specific IDs. */

type NamespaceId = DBId<NamespaceSpec>;
type ClassId = DBId<ClassSpec>;
type PropertyId = DBId<PropertySpec>;


#[derive(Serialize,Deserialize,JsonSchema,PartialEq,Eq,Debug)]
pub struct Input {
    
    /* WMI Objects */
    #[serde(rename = "Namespaces")]
    namespaces: HashMap<NamespaceId,NamespaceSpec>,
    #[serde(rename = "Classes")]
    classes: HashMap<ClassId,ClassSpec>,
    #[serde(rename = "Properties")]
    properties: HashMap<PropertyId,PropertySpec>,

    /* Data table and field mapping */
    #[serde(rename = "DataTables")]    
    data_tables: HashMap<DataTableId,ClassId>,
    #[serde(rename = "DataFields")]    
    data_fields: HashMap<DataFieldId,PropertyId>,

}


#[derive(DBObj,Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
struct NamespaceSpec {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Parent")]
    parent: Option<NamespaceId>,
}

#[derive(DBObj,Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
struct ClassSpec {
    #[serde(rename = "Namespace")]
    namespace: NamespaceId,
    #[serde(rename = "Name")]
    name: String,
}

#[derive(DBObj,Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
struct PropertySpec {
    #[serde(rename = "Class")]
    class: ClassId,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "IsKey")]
    is_key: bool,
    #[serde(rename = "Type")]
    typ: WMIType,    
}


#[derive(Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
enum WMIType {
    Integer,
    Float,
    String,
}


impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> Result<()> {
	self.namespaces.try_append(other.namespaces)?;
	self.classes.try_append(other.classes)?;
	self.properties.try_append(other.properties)?;
	self.data_tables.try_append(other.data_tables)?;
	self.data_fields.try_append(other.data_fields)?;
	Ok(())
    }
}

impl Default for Input {
    fn default() -> Self {
	Input {
	    namespaces: HashMap::new(),
	    classes: HashMap::new(),
	    properties: HashMap::new(),
	    data_tables: HashMap::new(),
	    data_fields: HashMap::new(),
	}
    }
}
