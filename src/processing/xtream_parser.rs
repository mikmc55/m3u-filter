use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use crate::create_m3u_filter_error_result;
use crate::m3u_filter_error::{M3uFilterError, M3uFilterErrorKind};

use crate::model::model_config::{default_as_empty_rc_str};
use crate::model::model_m3u::{PlaylistGroup, PlaylistItem, PlaylistItemHeader, XtreamCluster};

fn default_as_empty_list() -> Vec<PlaylistItem> { vec![] }

fn deserialize_number_from_string<'de, D, T: DeserializeOwned>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
{
    // we define a local enum type inside of the function
    // because it is untagged, serde will deserialize as the first variant
    // that it can
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaybeNumber<U> {
        // if it can be parsed as Option<T>, it will be
        Value(Option<U>),
        // otherwise try parsing as a string
        NumberString(String),
    }

    // deserialize into local enum
    let value: MaybeNumber<T> = Deserialize::deserialize(deserializer)?;
    match value {
        // if parsed as T or None, return that
        MaybeNumber::Value(value) => Ok(value),

        // (if it is any other string)
        MaybeNumber::NumberString(string) => {
            match serde_json::from_str::<T>(string.as_str()) {
                Ok(val) => Ok(Some(val)),
                Err(_) => Ok(None)
            }
        }
    }
}

fn value_to_string_array(value: &[Value]) -> Option<Vec<String>> {
    Some(value.iter().filter_map(value_to_string).collect())
}

fn value_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.to_string()),
        _ => None,
    }
}

fn deserialize_as_option_rc_string<'de, D>(deserializer: D) -> Result<Option<Rc<String>>, D::Error>
    where
        D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    match &value {
        Value::String(s) => Ok(Some(Rc::new(s.to_owned()))),
        Value::Number(s) => Ok(Some(Rc::new(s.to_string()))),
        _ => Ok(None),
    }
}

fn deserialize_as_rc_string<'de, D>(deserializer: D) -> Result<Rc<String>, D::Error>
    where
        D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    match &value {
        Value::String(s) => Ok(Rc::new(s.to_owned())),
        _ => Ok(Rc::new(value.to_string())),
    }
}

fn deserialize_as_string_array<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
    where
        D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(|v| match v {
        Value::Array(value) => value_to_string_array(&value),
        _ => None,
    })
}

#[derive(Deserialize)]
struct XtreamCategory {
    #[serde(deserialize_with = "deserialize_as_rc_string")]
    pub category_id: Rc<String>,
    #[serde(deserialize_with = "deserialize_as_rc_string")]
    pub category_name: Rc<String>,
    //pub parent_id: i32,
    #[serde(default = "default_as_empty_list")]
    pub channels: Vec<PlaylistItem>,
}

impl XtreamCategory {
    fn add(&mut self, item: PlaylistItem) {
        self.channels.push(item);
    }
}

#[derive(Serialize, Deserialize)]
struct XtreamStream {
    #[serde(default, deserialize_with = "deserialize_as_rc_string")]
    pub name: Rc<String>,
    #[serde(default, deserialize_with = "deserialize_as_rc_string")]
    pub category_id: Rc<String>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    pub stream_id: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    pub series_id: Option<i32>,
    #[serde(default = "default_as_empty_rc_str", deserialize_with = "deserialize_as_rc_string")]
    pub stream_icon: Rc<String>,
    #[serde(default = "default_as_empty_rc_str", deserialize_with = "deserialize_as_rc_string")]
    pub direct_source: Rc<String>,

    // optional attributes
    #[serde(default, deserialize_with = "deserialize_as_string_array")]
    backdrop_path: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    added: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    cast: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    container_extension: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    cover: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    director: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    episode_run_time: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    genre: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    last_modified: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    plot: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    rating: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    rating_5based: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    release_date: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    stream_type: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    title: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    year: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    youtube_trailer: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_as_option_rc_string")]
    epg_channel_id: Option<Rc<String>>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    tv_archive: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_number_from_string")]
    tv_archive_duration: Option<i32>,
}

macro_rules! add_str_property_if_exists {
    ($vec:expr, $prop:expr, $prop_name:expr) => {
       $prop.as_ref().map(|v| $vec.push((String::from($prop_name), Value::String(v.to_string()))));
    }
}
macro_rules! add_i64_property_if_exists {
    ($vec:expr, $prop:expr, $prop_name:expr) => {
       $prop.as_ref().map(|v| $vec.push((String::from($prop_name), Value::Number(serde_json::value::Number::from(i64::from(*v))))));
    }
}

macro_rules! add_f64_property_if_exists {
    ($vec:expr, $prop:expr, $prop_name:expr) => {
       $prop.as_ref().map(|v| $vec.push((String::from($prop_name), Value::Number(serde_json::value::Number::from_f64(f64::from(*v)).unwrap()))));
    }
}

impl XtreamStream {
    pub(crate) fn get_stream_id(&self) -> String {
        self.stream_id.map_or_else(|| self.series_id.map_or_else(|| String::from(""), |seid| format!("{}", seid)), |sid| format!("{}", sid))
    }

    pub(crate) fn get_additional_properties(&self) -> Option<Vec<(String, Value)>> {
        let mut result = vec![];
        if let Some(bdpath) = self.backdrop_path.as_ref() {
            if !bdpath.is_empty() {
                result.push((String::from("backdrop_path"), Value::Array(Vec::from([Value::String(String::from(bdpath.get(0).unwrap()))]))));
            }
        }
        add_str_property_if_exists!(result, self.added, "added");
        add_str_property_if_exists!(result, self.cast, "cast");
        add_str_property_if_exists!(result, self.container_extension, "container_extension");
        add_str_property_if_exists!(result, self.cover, "cover");
        add_str_property_if_exists!(result, self.director, "director");
        add_str_property_if_exists!(result, self.episode_run_time, "episode_run_time");
        add_str_property_if_exists!(result, self.genre, "genre");
        add_str_property_if_exists!(result, self.last_modified, "last_modified");
        add_str_property_if_exists!(result, self.plot, "plot");
        add_f64_property_if_exists!(result, self.rating, "rating");
        add_f64_property_if_exists!(result, self.rating_5based, "rating_5based");
        add_str_property_if_exists!(result, self.release_date, "release_date");
        add_str_property_if_exists!(result, self.stream_type, "stream_type");
        add_str_property_if_exists!(result, self.title, "title");
        add_str_property_if_exists!(result, self.year, "year");
        add_str_property_if_exists!(result, self.youtube_trailer, "youtube_trailer");
        //add_str_property_if_exists!(result, self.epg_channel_id, "epg_channel_id");
        add_i64_property_if_exists!(result, self.tv_archive, "tv_archive");
        add_i64_property_if_exists!(result, self.tv_archive_duration, "tv_archive_duration");
        if result.is_empty() { None } else { Some(result) }
    }
}

fn process_category(category: &Value) -> Result<Vec<XtreamCategory>, M3uFilterError> {
    match serde_json::from_value::<Vec<XtreamCategory>>(category.to_owned()) {
        Ok(category_list) => Ok(category_list),
        Err(err) => {
            create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Failed to process categories {}", &err)
        }
    }
}


fn process_streams(xtream_cluster: &XtreamCluster, streams: &Value) -> Result<Vec<XtreamStream>, M3uFilterError> {
    match serde_json::from_value::<Vec<XtreamStream>>(streams.to_owned()) {
        Ok(stream_list) => Ok(stream_list),
        Err(err) => {
            create_m3u_filter_error_result!(M3uFilterErrorKind::Notify, "Failed to process streams {:?}: {}", xtream_cluster, &err)
        }
    }
}

pub(crate) fn parse_xtream(cat_id_cnt: &AtomicI32,
                           xtream_cluster: &XtreamCluster,
                           category: &Value,
                           url: &str,
                           username: &str,
                           password: &str,
                           streams: &Value) -> Result<Option<Vec<PlaylistGroup>>, M3uFilterError> {
    match process_category(category) {
        Ok(mut categories) => {
            return match process_streams(xtream_cluster, streams) {
                Ok(streams) => {
                    let group_map: HashMap::<Rc<String>, RefCell<XtreamCategory>> =
                        categories.drain(..).map(|category|
                            (Rc::clone(&category.category_id), RefCell::new(category))
                        ).collect();

                    for stream in streams {
                        if let Some(group) = group_map.get(&stream.category_id) {
                            let mut grp = group.borrow_mut();
                            let title = &grp.category_name;
                            let item = PlaylistItem {
                                header: RefCell::new(PlaylistItemHeader {
                                    id: Rc::new(stream.get_stream_id()),
                                    name: Rc::clone(&stream.name),
                                    logo: Rc::clone(&stream.stream_icon),
                                    logo_small: default_as_empty_rc_str(),
                                    group: Rc::clone(title),
                                    title: Rc::clone(&stream.name),
                                    parent_code: default_as_empty_rc_str(),
                                    audio_track: default_as_empty_rc_str(),
                                    time_shift: default_as_empty_rc_str(),
                                    rec: default_as_empty_rc_str(),
                                    // source is meant to hold the original provider data
                                    source: default_as_empty_rc_str(),
                                    url: if stream.direct_source.is_empty() {
                                        let stream_base_url = match xtream_cluster {
                                            XtreamCluster::Live =>  format!("{}/live/{}/{}/{}.ts", url, username, password, &stream.get_stream_id()),
                                            XtreamCluster::Video => {
                                                let ext = stream.container_extension.as_ref().map_or("mp4", |e| e.as_str());
                                                format!("{}/live/{}/{}/{}.{}", url, username, password, &stream.get_stream_id(), ext)
                                            },
                                            XtreamCluster::Series =>
                                                format!("{}/player_api.php?username={}&password={}&action=get_series_info&series_id={}",
                                                        url, username, password, &stream.get_stream_id())
                                        };
                                        Rc::new(stream_base_url)
                                    } else {
                                        Rc::clone(&stream.direct_source)
                                    },
                                    epg_channel_id: stream.epg_channel_id.clone(),
                                    xtream_cluster: xtream_cluster.clone(),
                                    additional_properties: stream.get_additional_properties(),
                                }),
                            };
                            grp.add(item);
                        }
                    }

                    Ok(Some(group_map.values().map(|category| {
                        let cat = category.borrow();
                        cat_id_cnt.fetch_add(1, Ordering::Relaxed);
                        PlaylistGroup {
                            id: cat_id_cnt.load(Ordering::Relaxed),
                            xtream_cluster: xtream_cluster.clone(),
                            title: Rc::clone(&cat.category_name),
                            channels: cat.channels.clone(),
                        }
                    }).collect()))
                }
                Err(err) => Err(err)
            };
        }
        Err(err) => Err(err)
    }
}