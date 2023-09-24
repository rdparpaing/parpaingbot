use ::serenity::utils::MessageBuilder;
use dotenv::{dotenv, var};
use poise::serenity_prelude as serenity;
use rand::Rng;
use sqlx::{types::chrono, Connection, PgConnection, Row};
use tokio::sync::Mutex;
use url::Url;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

static ERROR_EMOJIS: [&str; 7] = [
    "<:mdmd:957638205442773063>",
    "<:chokbar:1145416431547203684>",
    "<:cas:1038561443185958972>",
    "<:bonkline:1082746112714227773>",
    "<:rireline:935485562687750175>",
    "<:wumboflush:931195078133821521>",
    "<:commentcamonreuf:1099314723255754844>",
];

struct Data {
    db: Mutex<PgConnection>,
}

struct Post {
    id: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    comment: Option<String>,
    attachment: Option<String>,
}

impl Post {
    async fn say(&self, ctx: Context<'_>) {
        let message_text = MessageBuilder::new()
            .push_italic_safe("Post N°")
            .push_bold_safe(self.id)
            .push_italic_safe(" créé le ")
            .push_bold_safe(self.created_at.format("%d/%m/%Y"))
            .push({
                if let Some(comment) = &self.comment {
                    if comment.is_empty() {
                        "".to_owned()
                    } else {
                        "\n> ".to_owned() + comment.as_str()
                    }
                } else {
                    "".to_owned()
                }
            })
            .build();

        ctx.send(|m| {
            if let Some(attachment) = &self.attachment {
                let url = Url::parse(attachment);

                if let Ok(url) = url {
                    m.attachment(serenity::AttachmentType::Image(url));
                }
            };

            m.content(message_text)
        })
        .await
        .unwrap();
    }
}

fn error(msg: &str) -> String {
    let mut rng = rand::thread_rng();

    let emoji = ERROR_EMOJIS[rng.gen_range(0..7)];

    format!("{} {}", emoji, msg)
}

/// On récupère les souvenirs hoplà
#[poise::command(prefix_command, slash_command)]
async fn chercher(
    ctx: Context<'_>,
    #[description = "L'identifiant ou l'alias du post"] id: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let mut db = ctx.data().db.lock().await;

    let id_num = id.parse::<i64>().unwrap_or(-1i64);

    let data = sqlx::query("SELECT * FROM archive WHERE id = $1 OR alias = $2")
        .bind(id_num)
        .bind(id)
        .fetch_one(&mut *db)
        .await;

    match data {
        Ok(data) => {
            let post = Post {
                id: data.get("id"),
                created_at: data.get("created_at"),
                comment: data.get("comment"),
                attachment: data.get("attachment"),
            };

            post.say(ctx).await;
        }
        Err(_) => {
            ctx.say(error("Le post n'a pas été trouvé")).await.unwrap();
        }
    }

    Ok(())
}

/// Risque de chokbar de bz
#[poise::command(slash_command)]
async fn aléatoire(
    ctx: Context<'_>,
    #[description = "La catégorie du post"] tag: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let mut db = ctx.data().db.lock().await;

    let data = sqlx::query(
        ("SELECT * FROM archive ".to_string()
            + match &tag {
                Some(_) => "WHERE tag = $1 ",
                None => "",
            }
            + "ORDER BY RANDOM() LIMIT 1")
            .as_str(),
    )
    .bind(tag)
    .fetch_one(&mut *db)
    .await;

    match data {
        Ok(data) => {
            let post = Post {
                id: data.get("id"),
                created_at: data.get("created_at"),
                comment: data.get("comment"),
                attachment: data.get("attachment"),
            };

            post.say(ctx).await;
        }
        Err(_) => {
            ctx.say(error("Aucun post n'a été trouvé")).await.unwrap();
        }
    }

    Ok(())
}

/// Allez on archive des trucs là
#[poise::command(slash_command)]
async fn créer(
    ctx: Context<'_>,
    #[description = "La categorie du post"] tag: String,
    #[description = "Un commentaire sur le post"] commentaire: Option<String>,
    #[description = "Une image pour le post"] fichier: Option<serenity::Attachment>,
    #[description = "Un alias pour accéder au post"] alias: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    if commentaire.is_none() && fichier.is_none() {
        ctx.say(error("Tu peux pas mettre un truc vide gros pd"))
            .await
            .unwrap();
        return Ok(());
    }

    let url = fichier.map(|f| f.url);

    let mut db = ctx.data().db.lock().await;
    let res = sqlx::query(
        "INSERT INTO archive (tag, comment, attachment, alias) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(tag)
    .bind(commentaire)
    .bind(url)
    .bind(alias)
    .fetch_one(&mut *db)
    .await;

    match res {
        Ok(data) => {
            ctx.say(
                ":sunglasses: Le post a été créé avec l'id : **".to_string()
                    + data.get::<i64, &str>("id").to_string().as_str()
                    + "**",
            )
            .await?
        }
        Err(_) => ctx.say(error("Aucun post n'a été créé batard")).await?,
    };

    Ok(())
}

/// ÇA DÉGAGE
#[poise::command(slash_command)]
async fn supprimer(
    ctx: Context<'_>,
    #[description = "L'identifiant du post"] id: i64,
) -> Result<(), Error> {
    ctx.defer().await?;

    let mut db = ctx.data().db.lock().await;

    let res = sqlx::query("DELETE FROM archive WHERE id = $1")
        .bind(id)
        .execute(&mut *db)
        .await?;

    if res.rows_affected() == 0 {
        ctx.say(error("Y'a rien qu'a bougé sale gros"))
            .await
            .unwrap();
    } else {
        ctx.say("Le post a été renvoyé dans son pays :flag_fr:")
            .await
            .unwrap();
    }

    Ok(())
}

#[poise::command(slash_command, subcommands("tag", "tout"))]
async fn liste(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Y'a quoi ici ? (ça liste les tags enculé)
#[poise::command(slash_command)]
async fn tout(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let mut db = ctx.data().db.lock().await;

    let mut tags: Vec<String> = sqlx::query("SELECT DISTINCT tag FROM archive")
        .fetch_all(&mut *db)
        .await?
        .iter()
        .map(|row| format!("``{}``", row.get::<String, &str>("tag")))
        .collect();

    tags.sort();
    let tags_text = tags.join(", ");

    ctx.say("Les tags sont : ".to_owned() + &tags_text).await?;

    Ok(())
}

/// Y'a quoi là-dedans ? (ça liste le contenu d'un tag)
#[poise::command(slash_command)]
async fn tag(ctx: Context<'_>, tag: String) -> Result<(), Error> {
    ctx.defer().await?;

    let mut db = ctx.data().db.lock().await;

    let mut tags: Vec<String> = sqlx::query("SELECT * FROM archive WHERE tag = $1")
        .bind(&tag)
        .fetch_all(&mut *db)
        .await?
        .iter()
        .map(|row| {
            let id = row.get::<i64, &str>("id").to_string();
            let alias = row.get::<Option<String>, &str>("alias");

            if let Some(alias) = alias {
                format!("``{}``", alias)
            } else {
                format!("``{}``", id)
            }
        })
        .collect();

    tags.sort_by(|a, b| a.cmp(b).reverse());
    let tags_text = tags.join(", ");

    ctx.say(format!("Les posts du tag ``{}`` sont : {}", &tag, tags_text).as_str())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load .env file");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![chercher(), aléatoire(), créer(), supprimer(), liste()],
            ..Default::default()
        })
        .token(var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                println!(
                    "Loaded {} commands globally",
                    framework.options().commands.len()
                );
                println!("Logged in as {}", ctx.cache.current_user().name);

                ctx.set_activity(serenity::Activity::watching("Breaking Bad "))
                    .await;

                Ok(Data {
                    db: Mutex::new(
                        PgConnection::connect(var("DATABASE_URL").unwrap().as_str())
                            .await
                            .unwrap(),
                    ),
                })
            })
        });

    framework.run().await.unwrap();
}
