use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::BigNeonError;
use bigneon_db::prelude::*;
use futures::future;
use log::Level;
use reqwest::blocking::Client;

pub struct SubmitSitemapToSearchEnginesExecutor {
    api_url: String,
    block_external_comms: bool,
}

impl DomainActionExecutor for SubmitSitemapToSearchEnginesExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Level::Trace, "Process submit to search engines failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl SubmitSitemapToSearchEnginesExecutor {
    pub fn new(api_url: String, block_external_comms: bool) -> SubmitSitemapToSearchEnginesExecutor {
        SubmitSitemapToSearchEnginesExecutor {
            api_url,
            block_external_comms,
        }
    }

    pub fn perform_job(&self, _action: &DomainAction, _conn: &Connection) -> Result<(), BigNeonError> {
        // block this function if environmental variable is set
        if self.block_external_comms {
            return Ok(());
        }
        ping_search_engines(&self.api_url)
    }
}

enum SearchEngine {
    Google,
    Bing,
}

fn get_search_engine_url(se: SearchEngine, api_url: &String) -> String {
    match se {
        SearchEngine::Google => format!(
            "http://www.google.com/webmasters/sitemaps/ping?sitemap={}/sitemap.xml",
            api_url
        ),
        SearchEngine::Bing => format!("http://www.bing.com/ping?sitemap={}/sitemap.xml", api_url),
    }
}

// Ping a search engine to read the sitemap url,
// do a http get to their giving url with the sitemap address to update
fn ping_search_engines(api_url: &String) -> Result<(), BigNeonError> {
    http_get(&get_search_engine_url(SearchEngine::Google, api_url))?;
    http_get(&get_search_engine_url(SearchEngine::Bing, api_url))?;
    Ok(())
}

// Http Get to a giving url, returning the body as a String
fn http_get(url: &String) -> Result<String, BigNeonError> {
    let client = Client::new();
    let s = client.get(url).send()?.text()?;
    Ok(s)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn search_engine_url() {
        let sitemap_url = "http://test.com/sitemap.com".to_string();
        let bing_result = get_search_engine_url(SearchEngine::Bing, &sitemap_url);
        let google_result = get_search_engine_url(SearchEngine::Google, &sitemap_url);
        let bing_result_test = format!("http://www.bing.com/ping?sitemap={}/sitemap.xml", sitemap_url);
        let google_result_test = format!(
            "http://www.google.com/webmasters/sitemaps/ping?sitemap={}/sitemap.xml",
            sitemap_url
        );
        assert_eq!(bing_result_test, bing_result);
        assert_eq!(google_result_test, google_result);
    }
}
