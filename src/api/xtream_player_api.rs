// https://github.com/tellytv/go.xtream-codes/blob/master/structs.go

use std::io::Error;
use std::str::FromStr;
use actix_web::{HttpRequest, HttpResponse, web, Resource};
use chrono::{Duration, Local};
use log::debug;

use crate::api::api_utils::{get_user_target, get_user_target_by_credentials, serve_file};
use crate::api::api_model::{AppState, UserApiRequest, XtreamAuthorizationResponse, XtreamServerInfo, XtreamUserInfo};
use crate::model::api_proxy::{UserCredentials};
use crate::model::config::{Config};
use crate::model::model_config::{TargetType};
use crate::repository::xtream_repository::{COL_CAT_LIVE, COL_CAT_SERIES, COL_CAT_VOD, COL_LIVE, COL_SERIES, COL_VOD, xtream_get_all, xtream_get_series_info, xtream_get_vod_info};
use crate::utils::get_client_request;

fn get_user_info(user: &UserCredentials, cfg: &Config) -> XtreamAuthorizationResponse {
    let server = cfg._api_proxy.read().unwrap().as_ref().unwrap().server.clone();
    let now = Local::now();
    XtreamAuthorizationResponse {
        user_info: XtreamUserInfo {
            active_cons: "0".to_string(),
            allowed_output_formats: Vec::from(["ts".to_string()]),
            auth: 1,
            created_at: (now - Duration::days(365)).timestamp(), // fake
            exp_date: (now + Duration::days(365)).timestamp(),// fake
            is_trial: "0".to_string(),
            max_connections: "1".to_string(),
            message: server.message.to_string(),
            password: user.password.to_string(),
            username: user.username.to_string(),
            status: "Active".to_string(),
        },
        server_info: XtreamServerInfo {
            url: server.ip.to_string(),
            port: server.http_port,
            https_port: server.https_port,
            server_protocol: server.protocol.clone(),
            rtmp_port: server.rtmp_port,
            timezone: server.timezone.to_string(),
            timestamp_now: now.timestamp(),
            time_now: now.format("%Y-%m-%d %H:%M:%S").to_string(),
        },
    }
}

async fn xtream_player_api_stream(
    api_req: &web::Query<UserApiRequest>,
    _app_state: &web::Data<AppState>,
    context: &str,
    username: &str,
    password: &str,
    stream_id: &str,
) -> HttpResponse {
    if let Some((_user, target)) = get_user_target_by_credentials(&username, &password, api_req, _app_state) {
        let target_name = &target.name;
        if target.has_output(&TargetType::Xtream) {
            match _app_state.config.get_xtream_input_for_target(target_name) {
                None => {}
                Some(input) => {
                    let username = input.username.as_ref().unwrap().clone();
                    let password = input.password.as_ref().unwrap().clone();
                    let stream_url = format!("{}/{}/{}/{}/{}", input.url, context, username, password, stream_id);
                    let url = reqwest::Url::parse(&stream_url).unwrap();
                    let client = get_client_request(input, url);
                    if let Ok(response) = client.send().await {
                        if response.status().is_success() {
                            return HttpResponse::Ok().streaming(response.bytes_stream());
                        }
                    }
                }
            }
        }
    }
    HttpResponse::BadRequest().finish()
}

async fn xtream_player_api_live_stream(
    api_req: web::Query<UserApiRequest>,
    path: web::Path<(String, String, String)>,
    _app_state: web::Data<AppState>,
) -> HttpResponse {
    let (username, password, stream_id) = path.into_inner();
    xtream_player_api_stream(&api_req, &_app_state, "live", &username, &password, &stream_id).await
}

async fn xtream_player_api_series_stream(
    api_req: web::Query<UserApiRequest>,
    path: web::Path<(String, String, String)>,
    _app_state: web::Data<AppState>,
) -> HttpResponse {
    let (username, password, stream_id) = path.into_inner();
    xtream_player_api_stream(&api_req, &_app_state, "series", &username, &password, &stream_id).await
}

async fn xtream_player_api_movie_stream(
    api_req: web::Query<UserApiRequest>,
    path: web::Path<(String, String, String)>,
    _app_state: web::Data<AppState>,
) -> HttpResponse {
    let (username, password, stream_id) = path.into_inner();
    xtream_player_api_stream(&api_req, &_app_state, "movie", &username, &password, &stream_id).await
}


async fn xtream_player_api(
    api_req: web::Query<UserApiRequest>,
    req: HttpRequest,
    _app_state: web::Data<AppState>,
) -> HttpResponse {
    match get_user_target(&api_req, &_app_state) {
        Some((user, target)) => {
            let action = api_req.action.trim();
            let target_name = &target.name;
            if target.has_output(&TargetType::Xtream) {
                if action.is_empty() {
                    return HttpResponse::Ok().json(get_user_info(&user, &_app_state.config));
                }

                match action {
                    "get_series_info" => {
                        match FromStr::from_str(api_req.series_id.trim()) {
                            Ok(stream_id) => {
                                match xtream_get_series_info(&_app_state.config, target_name, stream_id) {
                                    Ok(content) => HttpResponse::Ok().content_type(mime::APPLICATION_JSON).body(content),
                                    Err(_) => HttpResponse::NoContent().finish()
                                }
                            }
                            Err(_) => HttpResponse::BadRequest().finish()
                        }
                    }
                    "get_vod_info" => {
                        match FromStr::from_str(api_req.vod_id.trim()) {
                            Ok(stream_id) => {
                                match xtream_get_vod_info(&_app_state.config, target_name, stream_id) {
                                    Ok(content) => HttpResponse::Ok().content_type(mime::APPLICATION_JSON).body(content),
                                    Err(_) => HttpResponse::NoContent().finish()
                                }
                            }
                            Err(_) => HttpResponse::BadRequest().finish()
                        }
                    }
                    _ => {
                        match match action {
                            "get_live_categories" => xtream_get_all(&_app_state.config, target_name, COL_CAT_LIVE),
                            "get_vod_categories" => xtream_get_all(&_app_state.config, target_name, COL_CAT_VOD),
                            "get_series_categories" => xtream_get_all(&_app_state.config, target_name, COL_CAT_SERIES),
                            "get_live_streams" => xtream_get_all(&_app_state.config, target_name, COL_LIVE),
                            "get_vod_streams" => xtream_get_all(&_app_state.config, target_name, COL_VOD),
                            "get_series" => xtream_get_all(&_app_state.config, target_name, COL_SERIES),
                            _ => Err(Error::new(std::io::ErrorKind::Unsupported, format!("Cant find action: {}/{}", target_name, action))),
                        } {
                            Ok(maybe_file_path_or_content) => {
                                let (path, content) = maybe_file_path_or_content;
                                if let Some(file_path) = path {
                                    serve_file(&file_path, &req).await
                                } else if let Some(payload) = content {
                                    HttpResponse::Ok().body(payload)
                                } else {
                                    HttpResponse::NoContent().finish()
                                }
                            }
                            Err(err) => {
                                debug!("Could not open file for xtream target: {} {}", target_name, err);
                                HttpResponse::NoContent().finish()
                            }
                        }
                    }
                }
            } else {
                debug!("Target has not output xtream {}", target_name);
                HttpResponse::BadRequest().finish()
            }
        }
        _ => {
            if api_req.action.is_empty() {
                debug!("Paremeter action is empty!");
                HttpResponse::Unauthorized().finish()
            } else {
                debug!("cant find user!");
                HttpResponse::BadRequest().finish()
            }
        }
    }
}

pub(crate) fn xtream_api_register() -> Vec<Resource> {
    vec![
        web::resource("/player_api.php").route(web::get().to(xtream_player_api)),
        web::resource("/xtream").route(web::get().to(xtream_player_api)),
        web::resource("/live/{username}/{password}/{stream_id}").route(web::get().to(xtream_player_api_live_stream)),
        web::resource("/movie/{username}/{password}/{stream_id}").route(web::get().to(xtream_player_api_movie_stream)),
        web::resource("/series/{username}/{password}/{stream_id}").route(web::get().to(xtream_player_api_series_stream)),
    ]
}