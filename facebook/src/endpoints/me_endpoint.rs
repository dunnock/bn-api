use crate::endpoints::accounts_endpoint::AccountsEndpoint;
use crate::facebook_client::FacebookClientInner;
use std::rc::Rc;

pub struct MeEndpoint {
    pub accounts: AccountsEndpoint,
}

impl MeEndpoint {
    pub fn new(client: Rc<FacebookClientInner>) -> MeEndpoint {
        MeEndpoint {
            accounts: AccountsEndpoint { client },
        }
    }
}
