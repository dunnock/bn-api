use crate::error::ShareTribeError;
use crate::ResponseData;

pub type ShareTribeResult<T> = Result<ResponseData<T>, ShareTribeError>;
