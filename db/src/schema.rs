// @generated automatically by Diesel CLI.

diesel::table! {
    game_questions (id) {
        id -> Int4,
        game_id -> Int4,
        question_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    games (id) {
        id -> Int4,
        slug -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        creator -> Nullable<Text>,
    }
}

diesel::table! {
    questions (id) {
        id -> Int4,
        body -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    rounds (id) {
        id -> Int4,
        player_one -> Varchar,
        player_two -> Varchar,
        game_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        locked -> Bool,
        finished -> Bool,
    }
}

diesel::table! {
    user_questions (id) {
        id -> Int4,
        user_id -> Int4,
        question_id -> Int4,
        round_id -> Int4,
        answer -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
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

diesel::joinable!(game_questions -> games (game_id));
diesel::joinable!(game_questions -> questions (question_id));
diesel::joinable!(rounds -> games (game_id));
diesel::joinable!(user_questions -> questions (question_id));
diesel::joinable!(user_questions -> rounds (round_id));
diesel::joinable!(user_questions -> users (user_id));
diesel::joinable!(users -> games (game_id));

diesel::allow_tables_to_appear_in_same_query!(
    game_questions,
    games,
    questions,
    rounds,
    user_questions,
    users,
);
