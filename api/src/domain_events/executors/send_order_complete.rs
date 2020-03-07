use bigneon_db::prelude::*;
use crate::communications::mailers;
use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use futures::future;
use log::Level::Error;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SendOrderCompleteExecutor {
    config: Config,
}

impl DomainActionExecutor for SendOrderCompleteExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::new(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Send tickets mail action failed", {"action_id": action.id, "main_table_id":action.main_table_id,  "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::new(future::err(e)))
            }
        }
    }
}

impl SendOrderCompleteExecutor {
    pub fn new(config: Config) -> SendOrderCompleteExecutor {
        SendOrderCompleteExecutor { config }
    }

    pub fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        let order = Order::find(
            action
                .main_table_id
                .ok_or(ApplicationError::new("No order id supplied in the action".to_string()))?,
            conn,
        )?;
        let mut tokens_per_asset: HashMap<Uuid, Vec<u64>> = HashMap::new();
        let mut wallet_id_per_asset: HashMap<Uuid, Uuid> = HashMap::new();

        for oi in order.items(conn)? {
            let tickets = TicketInstance::find_for_order_item(oi.id, conn)?;
            let event = Event::find(oi.event_id.unwrap(), conn)?;

            let wallet = Wallet::find_default_for_organization(event.organization_id, conn)?;
            for ticket in tickets {
                tokens_per_asset
                    .entry(ticket.asset_id)
                    .or_insert_with(|| Vec::new())
                    .push(ticket.token_id as u64);
                wallet_id_per_asset.entry(ticket.asset_id).or_insert(wallet.id);
            }
        }

        let tari_client = self.config.tari_client.clone();

        let new_owner_wallet =
            Wallet::find_default_for_user(order.on_behalf_of_user_id.unwrap_or(order.user_id), conn)?;

        for (asset_id, token_ids) in &tokens_per_asset {
            let asset = Asset::find(*asset_id, conn)?;
            match asset.blockchain_asset_id {
                Some(a) => {
                    let wallet_id = match wallet_id_per_asset.get(asset_id) {
                        Some(w) => w.clone(),
                        None => {
                            return Err(ApplicationError::new(
                                "Could not complete this checkout because wallet id not found for asset".to_string(),
                            )
                            .into());
                        }
                    };
                    let org_wallet = Wallet::find(wallet_id, conn)?;
                    tari_client.transfer_tokens(
                        &org_wallet.secret_key,
                        &org_wallet.public_key,
                        &a,
                        token_ids.clone(),
                        new_owner_wallet.public_key.clone(),
                    )?
                }
                None => {
                    return Err(ApplicationError::new(
                        "Could not complete this checkout because the asset has not been assigned on the blockchain"
                            .to_string(),
                    )
                    .into());
                }
            }
        }

        let user = User::find(order.on_behalf_of_user_id.unwrap_or(order.user_id), conn)?;
        let display_order = order.for_display(None, user.id, conn)?;

        //Communicate purchase completed to user
        if let (Some(first_name), Some(email)) = (user.first_name, user.email) {
            mailers::orders::confirmation_email(&first_name, email, display_order, &self.config, conn)?.queue(conn)?;
        }
        Ok(())
    }
}
