use uuid::Uuid;

pub fn get_uuidv4() -> String {
    let uuid = Uuid::new_v4();
    let uuid_str = uuid.to_string().replace("-", "");
    return uuid_str;
}