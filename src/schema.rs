table! {
    game_questions (id) {
        id -> Int4,
        game_id -> Int4,
        question_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    games (id) {
        id -> Int4,
        creator -> Nullable<Text>,
        locked -> Bool,
        slug -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    questions (id) {
        id -> Int4,
        body -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int4,
        user_name -> Varchar,
        game_id -> Int4,
        session_id -> Nullable<Text>,
        score -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

joinable!(game_questions -> games (game_id));
joinable!(game_questions -> questions (question_id));
joinable!(users -> games (game_id));

allow_tables_to_appear_in_same_query!(
    game_questions,
    games,
    questions,
    users,
);
