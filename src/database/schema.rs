// @generated automatically by Diesel CLI.

diesel::table! {
    current_queue (id) {
        id -> Nullable<Integer>,
        guild_id -> Text,
        url -> Text,
        title -> Nullable<Text>,
        duration -> Nullable<Integer>,
        position -> Integer,
        added_by -> Text,
        added_at -> Timestamp,
    }
}

diesel::table! {
    guild_settings (guild_id) {
        guild_id -> Text,
        default_volume -> Float,
        auto_disconnect_minutes -> Integer,
        max_queue_size -> Integer,
        allowed_roles -> Nullable<Text>,
        blocked_domains -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    queue_history (id) {
        id -> Nullable<Integer>,
        guild_id -> Text,
        user_id -> Text,
        url -> Text,
        title -> Nullable<Text>,
        duration -> Nullable<Integer>,
        played_at -> Timestamp,
    }
}

diesel::table! {
    song_cache (url) {
        url -> Text,
        title -> Text,
        duration -> Nullable<Integer>,
        thumbnail_url -> Nullable<Text>,
        file_path -> Nullable<Text>,
        file_size -> Nullable<Integer>,
        last_accessed -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    voice_connections (guild_id) {
        guild_id -> Text,
        connected_at -> Timestamp,
        channel_id -> Nullable<Text>,
        last_activity -> Timestamp,
        current_track_title -> Nullable<Text>,
        is_playing -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    current_queue,
    guild_settings,
    queue_history,
    song_cache,
    voice_connections,
);
