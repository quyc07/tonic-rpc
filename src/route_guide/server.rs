use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::value::Serializer;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tonic::transport::Server;

use route_guide::{Feature, Point, Rectangle, RouteNote, RouteSummary};
use route_guide::route_guide_server::RouteGuide;
use route_guide::route_guide_server::RouteGuideServer;

mod data;
mod route_guide;

#[derive(Debug)]
pub struct RouteGuideService {
    features: Arc<Vec<Feature>>,
}

// pub mod route_guide {
// 当自定义了生成代码的位置后，可以直接导入生成的代码，不需要再指定包名
//     tonic::include_proto!("routeguide");
// }

#[tonic::async_trait]
impl RouteGuide for RouteGuideService {
    async fn get_feature(&self, request: Request<Point>) -> Result<Response<Feature>, Status> {
        Ok(Response::new(
            self.features
                .iter()
                .find(|x| x.location.as_ref() == Some(request.get_ref()))
                .map_or(Feature::default(), |t| t.clone()),
        ))
    }

    type ListFeaturesStream = ReceiverStream<Result<Feature, Status>>;

    async fn list_features(
        &self,
        request: Request<Rectangle>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        let (tx, rx) = mpsc::channel(4);
        let features = self.features.clone();
        tokio::spawn(async move {
            for x in features.to_vec() {
                if in_range(x.location.as_ref().unwrap(), request.get_ref()) {
                    tx.send(Ok(x.clone())).await.unwrap();
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn record_route(
        &self,
        request: Request<Streaming<Point>>,
    ) -> Result<Response<RouteSummary>, Status> {
        let mut stream = request.into_inner();
        let mut summary = RouteSummary::default();
        let mut last_point = None;
        let now = Instant::now();

        while let Some(point) = stream.next().await {
            let point = point?;
            summary.point_count += 1;
            for x in self.features.to_vec() {
                if x.location.as_ref() == Some(&point) {
                    summary.feature_count += 1;
                }
            }
            if let Some(ref last_point) = last_point {
                summary.distance += calc_distance(last_point, &point);
            }
            last_point = Some(point)
        }
        summary.elapsed_time = now.elapsed().as_secs() as i32;
        Ok(Response::new(summary))
    }

    type RouteChatStream = Pin<Box<dyn Stream<Item=Result<RouteNote, Status>> + Send + 'static>>;

    async fn route_chat(
        &self,
        request: Request<Streaming<RouteNote>>,
    ) -> Result<Response<Self::RouteChatStream>, Status> {
        let mut notes = HashMap::new();
        let mut stream = request.into_inner();
        let out_put = async_stream::try_stream! {
        while let Some(note) = stream.next().await {
            let note = note?;
            let location = note.location.clone().unwrap();
            let location_notes = notes.entry(location).or_insert(vec![]);
            location_notes.push(note);
            for note in location_notes {
                yield note.clone()
            }

        }

        };

        Ok(Response::new(Box::pin(out_put) as Self::RouteChatStream))
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.latitude.hash(state);
        self.longitude.hash(state);
    }
}

impl Eq for Point {}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:10000".parse().unwrap();
    let route_guide_service = RouteGuideService {
        features: Arc::new(data::load()),
    };
    let server = RouteGuideServer::new(route_guide_service);
    Server::builder()
        .add_service(server)
        .serve(addr)
        .await
        .unwrap();
}

fn in_range(point: &Point, rect: &Rectangle) -> bool {
    use std::cmp;

    let lo = rect.lo.as_ref().unwrap();
    let hi = rect.hi.as_ref().unwrap();

    let left = cmp::min(lo.longitude, hi.longitude);
    let right = cmp::max(lo.longitude, hi.longitude);
    let top = cmp::max(lo.latitude, hi.latitude);
    let bottom = cmp::min(lo.latitude, hi.latitude);

    point.longitude >= left
        && point.longitude <= right
        && point.latitude >= bottom
        && point.latitude <= top
}

/// Calculates the distance between two points using the "haversine" formula.
/// This code was taken from http://www.movable-type.co.uk/scripts/latlong.html.
fn calc_distance(p1: &Point, p2: &Point) -> i32 {
    const CORD_FACTOR: f64 = 1e7;
    const R: f64 = 6_371_000.0; // meters

    let lat1 = p1.latitude as f64 / CORD_FACTOR;
    let lat2 = p2.latitude as f64 / CORD_FACTOR;
    let lng1 = p1.longitude as f64 / CORD_FACTOR;
    let lng2 = p2.longitude as f64 / CORD_FACTOR;

    let lat_rad1 = lat1.to_radians();
    let lat_rad2 = lat2.to_radians();

    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lng = (lng2 - lng1).to_radians();

    let a = (delta_lat / 2f64).sin() * (delta_lat / 2f64).sin()
        + (lat_rad1).cos() * (lat_rad2).cos() * (delta_lng / 2f64).sin() * (delta_lng / 2f64).sin();

    let c = 2f64 * a.sqrt().atan2((1f64 - a).sqrt());

    (R * c) as i32
}
