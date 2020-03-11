use crate::payments::*;
use bigneon_db::models::PaymentProviders;
use chrono::NaiveDateTime;
use uuid::Uuid;

pub enum PaymentProcessorBehavior {
    AuthThenComplete(Box<dyn AuthThenCompletePaymentBehavior>),
    RedirectToPaymentPage(Box<dyn RedirectToPaymentPageBehavior>),
}

pub trait AuthThenCompletePaymentBehavior {
    fn payment_provider(&self) -> PaymentProviders;

    fn create_token_for_repeat_charges(
        &self,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError>;

    fn update_repeat_token(
        &self,
        repeat_token: &str,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError>;

    fn auth(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeAuthResult, PaymentProcessorError>;

    fn complete_authed_charge(&self, auth_token: &str) -> Result<ChargeResult, PaymentProcessorError>;
}

pub trait RedirectToPaymentPageBehavior {
    fn payment_provider(&self) -> PaymentProviders;
    fn create_payment_request(
        &self,
        total: f64,
        email: String,
        order_id: Uuid,
        ipn_url: Option<String>,

        success_url: Option<String>,
        cancel_url: Option<String>,
    ) -> Result<RedirectInfo, PaymentProcessorError>;
}

pub trait PaymentProcessor {
    fn behavior(&self) -> PaymentProcessorBehavior;
    fn refund(&self, auth_token: &str) -> Result<ChargeAuthResult, PaymentProcessorError>;

    fn partial_refund(&self, auth_token: &str, amount: i64) -> Result<ChargeAuthResult, PaymentProcessorError>;

    fn update_metadata(
        &self,
        charge_id: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<UpdateMetadataResult, PaymentProcessorError>;
}

#[derive(Serialize, Clone)]
pub struct RedirectInfo {
    pub redirect_url: String,
    pub id: String,
    pub expires_at: NaiveDateTime,
}
