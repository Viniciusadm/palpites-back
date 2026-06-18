use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use palpites_back::application::files::{
    CreateFileRecord, FileLinkRepository, FileRepository, FileStorage, FileUseCases, LinkFile,
    LinkUseCases, UploadFile,
};
use palpites_back::domain::files::File;
use palpites_back::domain::{DomainId, NonEmptyString, UtcDateTime};
use palpites_back::errors::AppError;

const MAX_BYTES: u64 = 5 * 1024 * 1024;

type LinkCalls = Arc<Mutex<Vec<(String, Option<String>)>>>;

#[tokio::test]
async fn upload_rejects_unsupported_content_type() {
    let use_cases = FileUseCases::new(FakeFiles::default(), FakeStorage::default(), MAX_BYTES);

    let result = use_cases
        .upload(UploadFile {
            owner_user_id: Some("user-1".to_owned()),
            content_type: "text/plain".to_owned(),
            original_name: "notes.txt".to_owned(),
            bytes: vec![1, 2, 3],
        })
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "unsupported_content_type",
            ..
        })
    ));
}

#[tokio::test]
async fn upload_rejects_oversize() {
    let use_cases = FileUseCases::new(FakeFiles::default(), FakeStorage::default(), 4);

    let result = use_cases
        .upload(UploadFile {
            owner_user_id: Some("user-1".to_owned()),
            content_type: "image/png".to_owned(),
            original_name: "avatar.png".to_owned(),
            bytes: vec![0u8; 10],
        })
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "file_too_large",
            ..
        })
    ));
}

#[tokio::test]
async fn upload_persists_metadata_and_returns_signed_url() {
    let files = FakeFiles::default();
    let storage = FakeStorage::default();
    let use_cases = FileUseCases::new(files.clone(), storage.clone(), MAX_BYTES);

    let stored = use_cases
        .upload(UploadFile {
            owner_user_id: Some("user-1".to_owned()),
            content_type: "image/png".to_owned(),
            original_name: "avatar.png".to_owned(),
            bytes: vec![10, 20, 30, 40],
        })
        .await
        .unwrap();

    let records = files.created.lock().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.owner_user_id.as_deref(), Some("user-1"));
    assert_eq!(record.bucket, "test-bucket");
    assert_eq!(record.content_type, "image/png");
    assert_eq!(record.byte_size, 4);
    assert_eq!(record.original_name, "avatar.png");
    assert!(record.checksum_sha256.is_some());
    assert_eq!(record.object_key, record.file_id);

    assert_eq!(stored.file.id.as_str(), record.file_id);
    assert_eq!(stored.url, format!("https://files.test/{}", record.object_key));

    let puts = storage.puts.lock().unwrap();
    assert_eq!(puts.len(), 1);
    assert_eq!(puts[0].1, "image/png");
    assert_eq!(puts[0].2, 4);
}

#[tokio::test]
async fn link_user_avatar_sets_fk() {
    let files = FakeFiles::with_existing("file-1");
    let links = FakeLinks::default();
    let use_cases = LinkUseCases::new(files, links.clone());

    use_cases
        .link_user_avatar(
            "user-1",
            LinkFile {
                file_id: Some("file-1".to_owned()),
            },
        )
        .await
        .unwrap();

    let calls = links.user_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "user-1");
    assert_eq!(calls[0].1.as_deref(), Some("file-1"));
}

#[tokio::test]
async fn link_team_flag_sets_fk() {
    let files = FakeFiles::with_existing("file-1");
    let links = FakeLinks::default();
    let use_cases = LinkUseCases::new(files, links.clone());

    use_cases
        .link_team_flag(
            "team-1",
            LinkFile {
                file_id: Some("file-1".to_owned()),
            },
        )
        .await
        .unwrap();

    let calls = links.team_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "team-1");
    assert_eq!(calls[0].1.as_deref(), Some("file-1"));
}

#[tokio::test]
async fn link_avatar_rejects_unknown_file() {
    let files = FakeFiles::default();
    let links = FakeLinks::default();
    let use_cases = LinkUseCases::new(files, links);

    let result = use_cases
        .link_user_avatar(
            "user-1",
            LinkFile {
                file_id: Some("missing".to_owned()),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "file_not_found",
            ..
        })
    ));
}

#[tokio::test]
async fn link_team_flag_rejects_unknown_team() {
    let files = FakeFiles::with_existing("file-1");
    let links = FakeLinks {
        missing_team: true,
        ..FakeLinks::default()
    };
    let use_cases = LinkUseCases::new(files, links);

    let result = use_cases
        .link_team_flag(
            "team-404",
            LinkFile {
                file_id: Some("file-1".to_owned()),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[derive(Clone, Default)]
struct FakeFiles {
    created: Arc<Mutex<Vec<CreateFileRecord>>>,
    existing_ids: Vec<String>,
}

impl FakeFiles {
    fn with_existing(id: &str) -> Self {
        Self {
            created: Arc::default(),
            existing_ids: vec![id.to_owned()],
        }
    }
}

#[async_trait]
impl FileRepository for FakeFiles {
    async fn create(&self, record: CreateFileRecord) -> Result<(), AppError> {
        self.created.lock().unwrap().push(record);
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<File>, AppError> {
        if self.existing_ids.iter().any(|existing| existing == id) {
            return Ok(Some(fake_file(id, None)));
        }
        let created = self.created.lock().unwrap();
        Ok(created
            .iter()
            .find(|record| record.file_id == id)
            .map(|record| fake_file(&record.file_id, record.owner_user_id.clone())))
    }
}

#[derive(Clone, Default)]
struct FakeStorage {
    puts: Arc<Mutex<Vec<(String, String, usize)>>>,
}

#[async_trait]
impl FileStorage for FakeStorage {
    fn bucket(&self) -> String {
        "test-bucket".to_owned()
    }

    async fn put(
        &self,
        object_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), AppError> {
        self.puts.lock().unwrap().push((
            object_key.to_owned(),
            content_type.to_owned(),
            bytes.len(),
        ));
        Ok(())
    }

    async fn signed_url(&self, object_key: &str) -> Result<String, AppError> {
        Ok(format!("https://files.test/{object_key}"))
    }

    async fn delete(&self, _object_key: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeLinks {
    user_calls: LinkCalls,
    team_calls: LinkCalls,
    missing_user: bool,
    missing_team: bool,
}

#[async_trait]
impl FileLinkRepository for FakeLinks {
    async fn set_user_avatar(
        &self,
        user_id: &str,
        file_id: Option<&str>,
    ) -> Result<bool, AppError> {
        self.user_calls
            .lock()
            .unwrap()
            .push((user_id.to_owned(), file_id.map(ToOwned::to_owned)));
        Ok(!self.missing_user)
    }

    async fn set_team_flag(&self, team_id: &str, file_id: Option<&str>) -> Result<bool, AppError> {
        self.team_calls
            .lock()
            .unwrap()
            .push((team_id.to_owned(), file_id.map(ToOwned::to_owned)));
        Ok(!self.missing_team)
    }
}

fn fake_file(id: &str, owner_user_id: Option<String>) -> File {
    File {
        id: DomainId::new(id.to_owned()).unwrap(),
        owner_user_id: owner_user_id.map(|value| DomainId::new(value).unwrap()),
        bucket: NonEmptyString::new("test-bucket".to_owned(), "file.bucket").unwrap(),
        object_key: NonEmptyString::new(id.to_owned(), "file.object_key").unwrap(),
        content_type: NonEmptyString::new("image/png".to_owned(), "file.content_type").unwrap(),
        byte_size: 4,
        original_name: NonEmptyString::new("avatar.png".to_owned(), "file.original_name").unwrap(),
        checksum_sha256: None,
        created_at: UtcDateTime::new_iso8601("2026-06-18T10:00:00").unwrap(),
    }
}
