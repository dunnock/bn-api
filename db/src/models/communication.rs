use diesel::PgConnection;
use itertools::Itertools;
use models::*;
use std::collections::HashMap;
use utils::errors::*;
use uuid::Uuid;

pub type TemplateData = HashMap<String, String>;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct CommAddress {
    pub addresses: Vec<String>,
}

impl CommAddress {
    pub fn new() -> CommAddress {
        CommAddress { addresses: Vec::new() }
    }

    pub fn from(address: String) -> CommAddress {
        CommAddress {
            addresses: vec![address],
        }
    }

    pub fn from_vec(addresses: Vec<String>) -> CommAddress {
        CommAddress { addresses }
    }

    pub fn get(&self) -> Vec<String> {
        self.addresses.clone()
    }

    pub fn get_first(&self) -> Result<String, DatabaseError> {
        if self.addresses.len() >= 1 {
            Ok(self.addresses[0].clone())
        } else {
            Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("Minimum of one communication address required".to_string()),
            ))
        }
    }

    pub fn push(&mut self, address: &String) {
        self.addresses.push(address.clone());
    }
}
#[derive(Serialize, Deserialize)]
pub struct Communication {
    pub comm_type: CommunicationType,
    pub title: String,
    pub body: Option<String>,
    pub source: Option<CommAddress>,
    pub destinations: CommAddress,
    pub template_id: Option<String>,
    pub template_data: Option<Vec<TemplateData>>,
    pub categories: Option<Vec<String>>,
    pub extra_data: Option<HashMap<String, Value>>,
    pub main_table: Option<Tables>,
    pub main_table_id: Option<Uuid>,
}

impl Communication {
    pub fn new<S: Into<String>>(
        comm_type: CommunicationType,
        title: String,
        body: Option<String>,
        source: Option<CommAddress>,
        destinations: CommAddress,
        template_id: Option<String>,
        template_data: Option<Vec<TemplateData>>,
        categories: Option<Vec<S>>,
        extra_data: Option<HashMap<String, Value>>,
    ) -> Communication {
        Communication {
            comm_type,
            title,
            body,
            source,
            destinations,
            template_id,
            template_data,
            categories: categories.map(|c| c.into_iter().map(|c1| c1.into()).collect_vec()),
            extra_data,
            main_table_id: None,
            main_table: None,
        }
    }

    pub fn queue(&self, connection: &PgConnection) -> Result<(), DatabaseError> {
        DomainAction::create(
            None,
            DomainActionTypes::Communication,
            match self.comm_type {
                CommunicationType::EmailTemplate => Some(CommunicationChannelType::Email),
                CommunicationType::Sms => Some(CommunicationChannelType::Sms),
                CommunicationType::Push => Some(CommunicationChannelType::Push),
                CommunicationType::Webhook => Some(CommunicationChannelType::Webhook),
            },
            json!(&self),
            self.main_table,
            self.main_table_id,
        )
        .commit(connection)?;
        Ok(())
    }
}
