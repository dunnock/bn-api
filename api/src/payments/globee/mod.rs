use crate::payments::*;
use bigneon_db::models::PaymentProviders;
use ::globee::*;
use uuid::Uuid;

pub struct GlobeePaymentProcessor {
    api_key: String,
    base_url: String,
}

impl GlobeePaymentProcessor {
    pub fn new(api_key: String, base_url: String) -> GlobeePaymentProcessor {
        GlobeePaymentProcessor { api_key, base_url }
    }
}

impl PaymentProcessor for GlobeePaymentProcessor {
    fn behavior(&self) -> PaymentProcessorBehavior {
        PaymentProcessorBehavior::RedirectToPaymentPage(Box::new(GlobeePaymentProcessorBehavior {
            client: GlobeeClient::new(self.api_key.clone(), self.base_url.clone()),
        }))
    }

    fn refund(&self, _auth_token: &str) -> Result<ChargeAuthResult, PaymentProcessorError> {
        Err(PaymentProcessorError {
            description: "Refunds are not supported by this gateway".to_string(),
            cause: None,
            validation_response: None,
        })
    }

    fn update_metadata(
        &self,
        _charge_id: &str,
        _metadata: Vec<(String, String)>,
    ) -> Result<UpdateMetadataResult, PaymentProcessorError> {
        Err(PaymentProcessorError {
            description: "Updating metadata is not supported by this gateway".to_string(),
            cause: None,
            validation_response: None,
        })
    }

    fn partial_refund(&self, _auth_token: &str, _amount: i64) -> Result<ChargeAuthResult, PaymentProcessorError> {
        Err(PaymentProcessorError {
            description: "Refunds are not supported by this gateway".to_string(),
            cause: None,
            validation_response: None,
        })
    }
}

pub struct GlobeePaymentProcessorBehavior {
    client: GlobeeClient,
}

impl RedirectToPaymentPageBehavior for GlobeePaymentProcessorBehavior {
    fn payment_provider(&self) -> PaymentProviders {
        PaymentProviders::Globee
    }

    fn create_payment_request(
        &self,
        amount: f64,
        email: String,
        payment_id: Uuid,
        ipn_url: Option<String>,
        success_url: Option<String>,
        cancel_url: Option<String>,
    ) -> Result<RedirectInfo, PaymentProcessorError> {
        let payment_request = PaymentRequest::new(
            amount,
            email,
            Some(payment_id.to_string()),
            ipn_url,
            success_url,
            cancel_url,
        );
        let result = self.client.create_payment_request(payment_request)?;
        Ok(RedirectInfo {
            id: result.id,
            redirect_url: result.redirect_url,
            expires_at: result.expires_at,
        })
    }
}

impl From<GlobeeError> for PaymentProcessorError {
    fn from(g: GlobeeError) -> Self {
        PaymentProcessorError {
            description: g.to_string(),
            cause: Some(Box::new(g)),
            validation_response: None,
        }
    }
}
