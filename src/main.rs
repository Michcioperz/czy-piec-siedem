use actix_web::Responder;
use quick_js::JsValue;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("got a value from doing javascript that wasn't quite what was expected")]
    JsValueMismatch,
    #[error("javascript execution error")]
    JsExecutionError(#[from] quick_js::ExecutionError),
    #[error("could not create javascript context")]
    JsContextError(#[from] quick_js::ContextError),
    #[error("failed to send http request")]
    RequestSendFailed(#[from] actix_web::client::SendRequestError),
    #[error("ssr script not found in response")]
    ScriptMissing,
    #[error("failed while reading server response")]
    ResponseBodyError(#[from] actix_web::client::PayloadError),
    #[error("failed to parse a time")]
    ParseTimeError(#[from] chrono::format::ParseError),
}

impl actix_web::error::ResponseError for Error {}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    actix_web::HttpServer::new(|| {
        actix_web::App::new()
            .data(actix_web::client::Client::default())
            .route("/api/357", actix_web::web::get().to(schedule_357))
            .route("/api/rns", actix_web::web::get().to(schedule_rns))
            .service(actix_files::Files::new("/", "frontend/public"))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

#[derive(Debug, serde::Serialize)]
struct Item357 {
    start_at: f64,
    end_at: f64,
    name: String,
    description: String,
    hosts: Vec<String>,
}

async fn schedule_rns(
    client: actix_web::web::Data<actix_web::client::Client>,
) -> Result<impl Responder, Error> {
    use chrono::Datelike;
    use select::predicate::{Child, Class, Name};
    let mut response = client
        .get("https://nowyswiat.online/ramowka/")
        .send()
        .await?;
    let body = response.body().await?;
    let doc =
        select::document::Document::from_read(&*body).expect("failed to read document from bytes");
    let mut items = vec![];
    let today = chrono::Local::today();
    for (day_of_week, tab) in doc.find(Class("proradio-tabs__content")).enumerate() {
        let day = today
            + chrono::Duration::days(
                (if day_of_week >= today.weekday().num_days_from_monday() as usize {
                    day_of_week
                } else {
                    day_of_week + 7
                } - today.weekday().num_days_from_monday() as usize) as i64,
            );
        for show in tab.find(Class("proradio-post__card--shows")) {
            let name = show
                .find(Child(Class("proradio-post__headercont--ex"), Name("h4")))
                .next()
                .or_else(|| {
                    show.find(Child(
                        Class("proradio-post__card__cap"),
                        Class("proradio-post__title"),
                    ))
                    .next()
                })
                .map(|node| node.text().trim().to_string())
                .unwrap_or_else(|| "".to_string());
            let description = show
                .find(Child(Class("proradio-post__headercont--ex"), Name("p")))
                .next()
                .map(|node| node.text().trim().to_string())
                .unwrap_or_else(|| "".to_string());
            let hosts = show
                .find(Child(Class("proradio-post__headercont--ex"), Name("h6")))
                .map(|node| node.text().trim().to_string())
                .collect();
            // TODO: better host splitting
            let times_string = show
                .find(Child(
                    Class("proradio-post__card__cap"),
                    Class("proradio-itemmetas"),
                ))
                .next()
                .map(|node| node.text())
                .unwrap_or_else(|| "00:00 - 00:00".to_string());
            let mut times = times_string.trim().split(" - ");
            let start_time =
                chrono::NaiveTime::parse_from_str(times.next().unwrap_or("00:00").trim(), "%H:%M")?;
            let end_time =
                chrono::NaiveTime::parse_from_str(times.next().unwrap_or("00:00").trim(), "%H:%M")?;
            let duration = end_time.signed_duration_since(start_time);
            let abs_duration = if duration > chrono::Duration::zero() {
                duration
            } else {
                -duration
            };
            let start_dt = day.and_time(start_time).unwrap();
            let end_dt = start_dt + abs_duration;
            items.push(Item357 {
                name,
                description,
                hosts,
                start_at: start_dt.timestamp_millis() as f64,
                end_at: end_dt.timestamp_millis() as f64,
            });
        }
    }
    Ok(actix_web::web::Json(items))
}

async fn schedule_357(client: actix_web::web::Data<actix_web::client::Client>) -> impl Responder {
    let mut response = client.get("https://radio357.pl/ramowka").send().await?;
    let body = response.body().await?;
    let doc =
        select::document::Document::from_read(&*body).expect("failed to read document from bytes");
    let script = doc
        .find(select::predicate::Child(
            select::predicate::Name("body"),
            select::predicate::Name("script"),
        ))
        .next()
        .ok_or(Error::ScriptMissing)?;
    let script_text = script.text();
    let context = quick_js::Context::new()?;
    context.eval("let window = {};")?;
    context.eval(&script_text)?;
    if let JsValue::Array(days) = context.eval("window.__NUXT__.state.schedule.schedule")? {
        let mut all_items = vec![];
        for day_value in days {
            if let JsValue::Object(day) = day_value {
                if let JsValue::Array(items) = day.get("items").ok_or(Error::JsValueMismatch)? {
                    for item_value in items {
                        if let JsValue::Object(item) = item_value {
                            match (
                                item.get("start_at"),
                                item.get("end_at"),
                                item.get("hosts"),
                                item.get("program"),
                            ) {
                                (
                                    Some(JsValue::Float(start_at)),
                                    Some(JsValue::Float(end_at)),
                                    Some(JsValue::Array(orig_hosts)),
                                    Some(JsValue::Object(program)),
                                ) => {
                                    let mut hosts = vec![];
                                    for orig_host_value in orig_hosts {
                                        if let JsValue::Object(orig_host) = orig_host_value {
                                            if let (
                                                Some(JsValue::String(first)),
                                                Some(JsValue::String(last)),
                                            ) = (
                                                orig_host.get("firstname"),
                                                orig_host.get("lastname"),
                                            ) {
                                                hosts.push(format!("{} {}", first, last));
                                            } else {
                                                return Err(Error::JsValueMismatch);
                                            }
                                        } else {
                                            return Err(Error::JsValueMismatch);
                                        }
                                    }
                                    if let (
                                        Some(JsValue::String(name)),
                                        Some(JsValue::String(description)),
                                    ) = (program.get("name"), program.get("description"))
                                    {
                                        all_items.push(Item357 {
                                            start_at: *start_at,
                                            end_at: *end_at,
                                            hosts,
                                            name: name.clone(),
                                            description: description.clone(),
                                        });
                                    } else {
                                        return Err(Error::JsValueMismatch);
                                    }
                                }
                                _ => return Err(Error::JsValueMismatch),
                            }
                        } else {
                            return Err(Error::JsValueMismatch);
                        }
                    }
                } else {
                    return Err(Error::JsValueMismatch);
                }
            } else {
                return Err(Error::JsValueMismatch);
            }
        }
        Ok(actix_web::web::Json(all_items))
    } else {
        Err(Error::JsValueMismatch)
    }
}
