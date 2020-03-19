use crate::support;
use actix_web::HttpResponse;
use api::helpers::application;

#[test]
fn unauthorized() {
    let response = application::unauthorized::<HttpResponse>(None, None);
    support::expects_unauthorized(&response.unwrap_err().into_inner().to_response());
}
