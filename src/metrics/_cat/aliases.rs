use elasticsearch::cat::CatAliasesParts;
use serde_json::Map as SerdeMap;

use super::responses::CatResponse;

pub(crate) const SUBSYSTEM: &str = "cat_aliases";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .aliases(CatAliasesParts::Name(&["*"]))
        .format("json")
        .h(&["*"])
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    let values = response
        .json::<CatResponse>()
        .await?
        .into_values(inject_cat_aliases_info);

    Ok(metric::from_values(values))
}

fn inject_cat_aliases_info(map: &mut SerdeMap<String, Value>) {
    let is_system_index: bool = map
        .get("index")
        .map(|index| index.as_str().map(|s| s.starts_with('.')).unwrap_or(false))
        .unwrap_or(false);

    if is_system_index {
        map.clear();
    } else {
        let _ = map.insert("info".into(), Value::from(1));
    }
}

crate::poll_metrics!();

#[tokio::test]
async fn test_cat_aliases() {
    let cat: CatResponse = serde_json::from_str(include_str!("../../tests/files/cat_aliases.json"))
        .expect("valid json");

    let got = cat.into_values(inject_cat_aliases_info);

    assert_eq!(got.len(), 52);

    assert_eq!(got[0]["info"], 1);
}
