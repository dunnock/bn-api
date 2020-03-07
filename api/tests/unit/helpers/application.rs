use actix_web::HttpResponse;
use bigneon_api::helpers::application;
use crate::support;

#[test]
fn unauthorized() {
    let response = application::unauthorized::<HttpResponse>(None, None);
    support::expects_unauthorized(&response.unwrap_err().into_inner().to_response());
}
