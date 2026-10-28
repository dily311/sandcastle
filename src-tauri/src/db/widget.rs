use crate::models::widget::*;
use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

pub async fn get_music_list<'e, E>(executor: E, folder_path: &str) -> Result<Vec<MusicMetaSummary>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, MusicMetaSummary>("
        SELECT path, album_art, updated_at
        FROM music_meta
        WHERE path LIKE ?
    ")
    .bind(&format!("{}%", folder_path))
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn get_all_music_data<'e, E>(executor: E) -> Result<Vec<MusicMeta>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, MusicMetaRow>("
        SELECT 
            mm.path,
            mm.title,
            mm.artist,
            mm.album,
            mm.album_title,
            mm.album_artist,
            mm.track_number,
            mm.genre,
            mm.comment,
            mm.album_art,
            (
                SELECT pos 
                FROM music_album_art_pos mp
                WHERE mm.path = mp.music_src
            ) AS img_pos,
            mm.lyrics,
            mm.duration,
            mm.updated_at
        FROM music_meta mm 
    ")
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_folder_music_data<'e, E>(executor: E, folder_path: &str) -> Result<Vec<MusicMeta>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, MusicMetaRow>("
        SELECT 
            mm.path,
            mm.title,
            mm.artist,
            mm.album,
            mm.album_title,
            mm.album_artist,
            mm.track_number,
            mm.genre,
            mm.comment,
            mm.album_art,
            (
                SELECT pos 
                FROM music_album_art_pos mp
                WHERE mm.path = mp.music_src
            ) AS img_pos,
            mm.lyrics,
            mm.duration,
            mm.updated_at
        FROM music_meta mm 
        WHERE mm.path LIKE ?
    ")
    .bind(&format!("{}%", folder_path))
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_playlist_entry_data<'e, E>(executor: E, playlist_id: &str) -> Result<Vec<MusicMeta>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, MusicMetaRow>("
        SELECT 
            mm.path,
            mm.title,
            mm.artist,
            mm.album,
            mm.album_title,
            mm.album_artist,
            mm.track_number,
            mm.genre,
            mm.comment,
            mm.album_art,
            (
                SELECT pos 
                FROM music_album_art_pos mp
                WHERE mm.path = mp.music_src
            ) AS img_pos,
            mm.lyrics,
            mm.duration,
            mm.updated_at
        FROM music_meta mm 
        INNER JOIN playlist_entries pm ON mm.path = pm.music_src
        WHERE pm.playlist_id = ?
        ORDER BY pm.sort_order DESC
    ")
    .bind(playlist_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn upsert_music_meta<'e, E>(executor: E, input: MusicMetaInput) -> Result<MusicMeta, Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "
        INSERT INTO music_meta (path, title, artist, album, album_title, album_artist, track_number, genre, comment, album_art, lyrics, duration, updated_at)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)
        ON CONFLICT(path) DO UPDATE SET
            title=excluded.title, 
            artist=excluded.artist, 
            album=excluded.album, 
            album_title=excluded.album_title, 
            album_artist=excluded.album_artist, 
            track_number=excluded.track_number, 
            genre=excluded.genre, 
            comment=excluded.comment, 
            album_art=excluded.album_art, 
            lyrics=excluded.lyrics, 
            duration=excluded.duration, 
            updated_at=excluded.updated_at
        "
    )
    .bind(&input.path)
    .bind(&input.title)
    .bind(&input.artist)
    .bind(&input.album)
    .bind(&input.album_title)
    .bind(&input.album_artist)
    .bind(&input.track_number)
    .bind(&input.genre)
    .bind(&input.comment)
    .bind(&input.album_art)
    .bind(&input.lyrics)
    .bind(&input.duration)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(MusicMeta {
        path: input.path,
        title: input.title,
        artist: input.artist,
        album: input.album,
        album_title: input.album_title,
        album_artist: input.album_artist,
        track_number: input.track_number,
        genre: input.genre,
        comment: input.comment,
        album_art: input.album_art,
        lyrics: input.lyrics,
        duration: input.duration,
        updated_at: now,
        img_pos: None,
    })
}

pub async fn delete_music_meta<'e, E>(executor: E, path: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM music_meta WHERE path = ?")
        .bind(path)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn upsert_music_album_art_pos<'e, E>(executor: E, path: &str, pos: serde_json::Value) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let pos_json = serde_json::to_string(&pos).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO music_album_art_pos ( music_src, pos )
        VALUES (?, ?)
        ON CONFLICT(music_src) DO UPDATE SET
            pos=excluded.pos
        "#,
    )
    .bind(path)
    .bind(pos_json)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_all_playlist<'e, E>(executor: E) -> Result<Vec<Playlist>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, Playlist>("
        SELECT * FROM playlist ORDER BY sort_order DESC
    ")
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn create_playlist<'e, E>(executor: E, input: PlaylistInput) -> Result<Playlist, Error> where E: Executor<'e, Database = Sqlite> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query("
        INSERT INTO playlist (id, title, description, cover_image, sort_order, created_at, updated_at)
        VALUES (?,?,?,?,?,?,?)
    ")
    .bind(&id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.cover_image)
    .bind(&input.sort_order)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(Playlist { 
        id,
        title: input.title,
        description: input.description,
        cover_image: input.cover_image,
        sort_order: input.sort_order,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_playlist<'e, E>(executor: E, id: &str, input: PlaylistInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("
        UPDATE playlist 
        SET
            title=?,
            description=?,
            cover_image=?,
            sort_order=?,
            updated_at=?
        WHERE id =?
    ")
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.cover_image)
    .bind(&input.sort_order)
    .bind(now)
    .bind(id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn delete_playlist<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM playlist WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn append_playlist_entry<'e, E>(executor: E, playlist_id: &str, music_src: &str, sort_order: Option<u32>,) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("INSERT INTO playlist_entries (playlist_id, music_src, sort_order) VALUES (?, ?, ?)")
    .bind(playlist_id)
    .bind(music_src)
    .bind(sort_order.unwrap_or(0))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_all_playlist_entry<'e, E>(executor: E, playlist_id: &str) -> Result<Vec<PlaylistItem>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, PlaylistItemRow>(
        r#"
        SELECT 
            mm.path,
            mm.title,
            mm.artist,
            mm.album,
            mm.album_title,
            mm.album_artist,
            mm.track_number,
            mm.genre,
            mm.comment,
            mm.album_art,
            (
                SELECT pos 
                FROM music_album_art_pos mp
                WHERE mm.path = mp.music_src
            ) AS img_pos,
            mm.lyrics,
            mm.duration,
            mm.updated_at,
            pe.sort_order 
        FROM playlist_entries pe
        JOIN music_meta mm ON pe.music_src = mm.path
        WHERE pe.playlist_id=?
        "#,
    )
    .bind(playlist_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn update_playlist_music_order<'e, E>(executor: E, playlist_id: &str, music_src: &str, sort_order: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query(
        r#"
        UPDATE playlist_entries SET sort_order =? WHERE playlist_id=? AND music_src =?
        "#,
    )
    .bind(sort_order)
    .bind(playlist_id)
    .bind(music_src)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn remove_playlist_entry<'e, E>(executor: E, playlist_id: &str, music_src: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM playlist_entries WHERE playlist_id = ? AND music_src=?")
        .bind(playlist_id)
        .bind(music_src)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}