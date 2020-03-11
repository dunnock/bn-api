use crate::payments::*;
use bigneon_db::models::PaymentProviders;
use stripe::StripeClient;
use stripe::StripeError;

impl From<StripeError> for PaymentProcessorError {
    fn from(s: StripeError) -> PaymentProcessorError {
        let validation_response = match s.error_code {
            Some(ref error_code) => match error_code.as_ref() {
                "card_declined" => Some("Card has been declined"),
                "expired_card" => Some("Card expired"),
                "incorrect_address" => Some("Incorrect address"),
                "incorrect_cvc" => Some("Incorrect CVC"),
                "incorrect_number" => Some("Incorrect number"),
                "incorrect_zip" => Some("Incorrect ZIP"),
                "invalid_card_type" => Some("Invalid card type"),
                "invalid_cvc" => Some("Invalid CVC"),
                "invalid_expiry_month" => Some("Invalid card expiry month"),
                "invalid_expiry_year" => Some("Invalid card expiry year"),
                "invalid_number" => Some("Invalid card number"),
                "balance_insufficient" => Some("Balance insufficient"),
                _ => None,
            },
            None => None,
        }
        .map(|s| s.to_string());

        PaymentProcessorError {
            description: s.description.clone(),
            cause: Some(Box::new(s)),
            validation_response,
        }
    }
}

pub struct StripePaymentProcessor {
    client: StripeClient,
}

impl StripePaymentProcessor {
    pub fn new(stripe_secret_key: String) -> StripePaymentProcessor {
        StripePaymentProcessor {
            client: StripeClient::new(stripe_secret_key),
        }
    }
}

pub struct StripePaymentBehavior {
    client: StripeClient,
}

impl PaymentProcessor for StripePaymentProcessor {
    fn behavior(&self) -> PaymentProcessorBehavior {
        PaymentProcessorBehavior::AuthThenComplete(Box::new(StripePaymentBehavior {
            client: self.client.clone(),
        }))
    }

    fn update_metadata(
        &self,
        charge_id: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<UpdateMetadataResult, PaymentProcessorError> {
        Ok(self
            .client
            .update_metadata(charge_id, metadata)
            .map(|r| UpdateMetadataResult {
                id: r.id,
                raw: r.raw_data,
            })?)
    }

    fn refund(&self, auth_token: &str) -> Result<ChargeAuthResult, PaymentProcessorError> {
        Ok(self.client.refund(auth_token).map(|r| ChargeAuthResult {
            id: r.id,
            raw: r.raw_data,
        })?)
    }

    fn partial_refund(&self, auth_token: &str, amount: i64) -> Result<ChargeAuthResult, PaymentProcessorError> {
        Ok(self
            .client
            .partial_refund(auth_token, amount)
            .map(|r| ChargeAuthResult {
                id: r.id,
                raw: r.raw_data,
            })?)
    }
}

impl<'a> AuthThenCompletePaymentBehavior for StripePaymentBehavior {
    fn payment_provider(&self) -> PaymentProviders {
        PaymentProviders::Stripe
    }
    fn create_token_for_repeat_charges(
        &self,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError> {
        Ok(self
            .client
            .create_customer(description, token, Vec::<(String, String)>::new())
            .map(|r| RepeatChargeToken {
                token: r.id,
                raw: r.raw_data,
            })?)
    }

    fn update_repeat_token(
        &self,
        repeat_token: &str,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError> {
        Ok(self
            .client
            .update_customer(repeat_token, description, token, Vec::<(String, String)>::new())
            .map(|r| RepeatChargeToken {
                token: r.id,
                raw: r.raw_data,
            })?)
    }

    fn auth(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeAuthResult, PaymentProcessorError> {
        Ok(self
            .client
            .auth(token, amount, currency, description, metadata)
            .map(|r| ChargeAuthResult {
                id: r.id,
                raw: r.raw_data,
            })?)
    }

    fn complete_authed_charge(&self, _auth_token: &str) -> Result<ChargeResult, PaymentProcessorError> {
        Ok(self.client.complete(_auth_token).map(|r| ChargeResult {
            id: r.id,
            raw: r.raw_data,
        })?)
    }
}
