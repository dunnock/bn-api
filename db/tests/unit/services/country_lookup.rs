use db::prelude::*;

#[test]
fn new() {
    let country_lookup = CountryLookup::new().unwrap();
    assert_eq!(country_lookup.country_data.len(), 242);
}

#[test]
fn find() {
    let country_lookup = CountryLookup::new().unwrap();
    assert!(country_lookup.find("FAKE").is_none());

    let country = country_lookup.find("US").unwrap();
    assert_eq!(country.code, "US");
    assert_eq!(country.name, "United States");

    let country = country_lookup.find("zA").unwrap();
    assert_eq!(country.code, "ZA");
    assert_eq!(country.name, "South Africa");

    let country = country_lookup.find("JapaN").unwrap();
    assert_eq!(country.code, "JP");
    assert_eq!(country.name, "Japan");
}

#[test]
fn parse_city_state() {
    let country_lookup = CountryLookup::new().unwrap();
    let us_country = country_lookup.find("US").unwrap();
    let jp_country = country_lookup.find("JP").unwrap();

    assert!(us_country.parse_city_state("FAKE").is_err());

    assert_eq!(
        us_country.parse_city_state("MA"),
        Ok(vec![(None, us_country.state("Massachusetts"))])
    );

    assert_eq!(
        us_country.parse_city_state("MaSSaChuseTTs"),
        Ok(vec![(None, us_country.state("Massachusetts"))])
    );

    assert_eq!(
        jp_country.parse_city_state("Aomori"),
        Ok(vec![(None, jp_country.state("JP-02"))])
    );

    assert_eq!(
        jp_country.parse_city_state("Test Aomori"),
        Ok(vec![(Some("Test".to_string()), jp_country.state("JP-02"))])
    );
}

#[test]
fn parse_city_state_country() {
    let country_lookup = CountryLookup::new().unwrap();
    let us_country = country_lookup.find("US").unwrap();
    let jp_country = country_lookup.find("JP").unwrap();

    assert!(country_lookup.parse_city_state_country("FAKE").is_err());

    assert_eq!(
        country_lookup.parse_city_state_country("US"),
        Ok(vec![(None, None, Some(us_country.clone()))])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("MA US"),
        Ok(vec![(
            None,
            us_country.state("Massachusetts"),
            Some(us_country.clone())
        )])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("MA"),
        Ok(vec![(None, None, Some(country_lookup.find("Morocco").unwrap()))])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("MaSSaChuseTTs US"),
        Ok(vec![(
            None,
            us_country.state("Massachusetts"),
            Some(us_country.clone())
        )])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("JapaN"),
        Ok(vec![(None, None, Some(jp_country.clone()))])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("Aomori JapaN"),
        Ok(vec![(None, jp_country.state("JP-02"), Some(jp_country.clone()))])
    );

    assert_eq!(
        country_lookup.parse_city_state_country("Test Aomori JapaN"),
        Ok(vec![(
            Some("Test".to_string()),
            jp_country.state("JP-02"),
            Some(jp_country.clone())
        )])
    );
}

#[test]
fn state() {
    let country_lookup = CountryLookup::new().unwrap();
    let country = country_lookup.find("US").unwrap();
    assert!(country.state("FAKE").is_none());

    let state = country.state("MA").unwrap();
    assert_eq!(state.name, "Massachusetts");
    assert_eq!(state.code, Some("MA".to_string()));
    let state = country.state("MaSSaChuseTTs").unwrap();
    assert_eq!(state.name, "Massachusetts");
    assert_eq!(state.code, Some("MA".to_string()));

    let country = country_lookup.find("JapaN").unwrap();
    let state = country.state("JP-02").unwrap();
    assert_eq!(state.name, "Aomori");
    assert_eq!(state.code, Some("JP-02".to_string()));
}
