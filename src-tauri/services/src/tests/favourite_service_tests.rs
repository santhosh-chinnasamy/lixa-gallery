use super::fakes::*;
use crate::favourite_service::FavouriteService;
use std::sync::Arc;
use std::sync::Mutex;

#[tokio::test]
async fn test_favourite_flow() {
    let repo = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo.clone(), fs);

    service
        .add_favourite("/path/to/photo.jpg".to_string())
        .await
        .unwrap();

    let favs = service.get_favourites().await.unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].path, "/path/to/photo.jpg");

    service
        .remove_favourite("/path/to/photo.jpg".to_string())
        .await
        .unwrap();
    let favs = service.get_favourites().await.unwrap();
    assert_eq!(favs.len(), 0);
}
