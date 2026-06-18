use crate::domain::files::File;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadFile {
    pub owner_user_id: Option<String>,
    pub content_type: String,
    pub original_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    pub file: File,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFile {
    pub file_id: Option<String>,
}
