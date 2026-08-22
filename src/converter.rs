use crate::model::Travel;
use crate::slint_generatedMainWindow as v;
use crate::{TravelItem, model as m};
use slint::{Image, ModelRc, SharedPixelBuffer, ToSharedString, VecModel};
use std::error::Error;

pub trait Converter<R> {
    fn convert(&self) -> Result<R, Box<dyn Error>>;
}

pub trait Mapper<I, R> {
    fn convert(&self, r: &I) -> R;
}

impl Converter<Image> for Vec<u8> {
    fn convert(&self) -> Result<Image, Box<dyn Error>> {
        let buffer = match image::load_from_memory(self) {
            Ok(img) => {
                SharedPixelBuffer::clone_from_slice(img.as_bytes(), img.width(), img.height())
            }
            Err(err) => return Err(Box::new(err)),
        };

        Ok(Image::from_rgba8(buffer))
    }
}

pub struct TravelDefaults {
    pub cover: Image,
}

impl Mapper<m::Travel, v::TravelItem> for TravelDefaults {
    fn convert(&self, travel: &Travel) -> TravelItem {
        v::TravelItem {
            city: travel.city.to_shared_string(),
            country: travel.country.to_shared_string(),
            cover: match &travel.cover {
                None => self.cover.clone(),
                Some(photo) => match photo.data.convert() {
                    Ok(img) => img,
                    Err(_) => self.cover.clone(),
                },
            },
            date: travel.began.to_shared_string() + " - " + travel.ended.to_string().as_str(),
            name: "".to_shared_string(),
            tags: ModelRc::new(VecModel::from_iter(
                vec!["tag1", "tag2", "tag3", "tag4"]
                    .iter()
                    .map(|e| e.to_shared_string()),
            )),
        }
    }
}
