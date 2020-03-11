use crate::BranchDeepLinker;
use crate::DeepLinker;
use bigneon_config::Config;
use bigneon_errors::*;
use bigneon_payments::globee::GlobeePaymentProcessor;
use bigneon_payments::stripe::StripePaymentProcessor;
use bigneon_payments::PaymentProcessor;
use bigneon_db::prelude::*;
use bigneon_db::services::CountryLookup;
use bigneon_db::utils::errors::DatabaseError;

pub struct ServiceLocator {
    stripe_secret_key: String,
    globee_api_key: String,
    globee_base_url: String,
    branch_io_base_url: String,
    branch_io_branch_key: String,
    branch_io_timeout: u64,
    api_keys_encryption_key: String,
    country_lookup_service: CountryLookup,
}

impl ServiceLocator {
    pub fn new(config: &Config) -> Result<ServiceLocator, DatabaseError> {
        let country_lookup_service = if config.environment == Environment::Test {
            CountryLookup {
                country_data: Vec::new(),
            }
        } else {
            CountryLookup::new()?
        };
        Ok(ServiceLocator {
            stripe_secret_key: config.stripe_secret_key.clone(),
            globee_api_key: config.globee_api_key.clone(),
            globee_base_url: config.globee_base_url.clone(),
            branch_io_base_url: config.branch_io_base_url.clone(),
            branch_io_branch_key: config.branch_io_branch_key.clone(),
            branch_io_timeout: config.branch_io_timeout,
            api_keys_encryption_key: config.api_keys_encryption_key.clone(),
            country_lookup_service,
        })
    }

    pub fn country_lookup_service(&self) -> &CountryLookup {
        &self.country_lookup_service
    }

    pub fn create_payment_processor(
        &self,
        provider: PaymentProviders,
        organization: &Organization,
    ) -> Result<Box<dyn PaymentProcessor>, BigNeonError> {
        match provider {
            PaymentProviders::Stripe => Ok(Box::new(StripePaymentProcessor::new(self.stripe_secret_key.clone()))),
            PaymentProviders::Globee => {
                let mut org = organization.clone();
                org.decrypt(&self.api_keys_encryption_key)?;

                let api_key = org.globee_api_key.as_ref().unwrap_or(&self.globee_api_key);
                Ok(Box::new(GlobeePaymentProcessor::new(
                    api_key.to_string(),
                    self.globee_base_url.clone(),
                )))
            }
            // External is not valid for service locator
            PaymentProviders::Free | PaymentProviders::External => {
                return Err(ApplicationError::new("Unknown payment provider".into()).into());
            }
        }
    }

    pub fn create_deep_linker(&self) -> Result<Box<dyn DeepLinker>, BigNeonError> {
        Ok(Box::new(BranchDeepLinker::new(
            self.branch_io_base_url.clone(),
            self.branch_io_branch_key.clone(),
            self.branch_io_timeout,
        )))
    }

    pub fn is_refund_supported(provider: String) -> bool {
        match provider.to_lowercase().as_str() {
            "stripe" => true,
            "globee" => false,
            "external" => false,
            _ => false,
        }
    }
}
