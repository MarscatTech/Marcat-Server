#[macro_use]
extern crate ic_cdk_macros;

mod auth;
mod storage;
mod types;
mod upload;

use candid::{CandidType, Principal};
use serde::Deserialize;
use storage::{ADMINS, COMMENTS, COMMENT_REPLIES, LIKES, POSTS, POST_COMMENTS, USERS};
use types::{
    Comment, CommentReplyKey, Error, LikeKey, Post, PostCommentKey, PostImage, PostStatus,
    PostSummary, PostsPage, UserProfile, MAX_COMMENT_LENGTH, MAX_CONTENT_LENGTH,
    MAX_IMAGES_PER_POST,
};

#[derive(CandidType, Deserialize)]
struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(CandidType)]
struct HttpResponse {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[init]
fn init() {
    auth::init_admin(ic_cdk::caller());
}

#[post_upgrade]
fn post_upgrade() {}

#[update]
fn add_admin(principal: Principal) -> Result<(), Error> {
    auth::require_admin()?;
    ADMINS.with(|a| {
        a.borrow_mut().insert(principal, ());
    });
    Ok(())
}

#[update]
fn remove_admin(principal: Principal) -> Result<(), Error> {
    auth::require_admin()?;
    ADMINS.with(|a| {
        a.borrow_mut().remove(&principal);
    });
    Ok(())
}

#[update]
fn register(nickname: String, avatar_url: String) -> Result<(), Error> {
    let caller = ic_cdk::caller();

    if USERS.with(|u| u.borrow().contains_key(&caller)) {
        return Err(Error::UserAlreadyExists);
    }

    let profile = UserProfile {
        principal: caller,
        nickname,
        avatar_url,
        created_at: ic_cdk::api::time(),
    };

    USERS.with(|u| {
        u.borrow_mut().insert(caller, profile);
    });
    Ok(())
}

#[update]
fn update_profile(nickname: Option<String>, avatar_url: Option<String>) -> Result<(), Error> {
    let caller = ic_cdk::caller();

    USERS.with(|u| {
        let mut users = u.borrow_mut();
        let mut profile = users.get(&caller).ok_or(Error::UserNotFound)?;

        if let Some(n) = nickname {
            profile.nickname = n;
        }
        if let Some(a) = avatar_url {
            profile.avatar_url = a;
        }

        users.insert(caller, profile);
        Ok(())
    })
}

#[query]
fn get_profile(principal: Principal) -> Result<UserProfile, Error> {
    USERS.with(|u| u.borrow().get(&principal).ok_or(Error::UserNotFound))
}

#[update]
fn delete_profile() -> Result<(), Error> {
    let caller = ic_cdk::caller();
    USERS.with(|u| {
        if u.borrow_mut().remove(&caller).is_none() {
            return Err(Error::UserNotFound);
        }
        Ok(())
    })
}

#[derive(CandidType, Deserialize)]
struct CreatePostRequest {
    content: String,
    images: Vec<(String, u64, u32)>,
}

#[update]
fn create_post(req: CreatePostRequest) -> Result<u64, Error> {
    auth::require_registered()?;

    if req.content.len() > MAX_CONTENT_LENGTH {
        return Err(Error::ContentTooLong);
    }
    if req.images.len() > MAX_IMAGES_PER_POST as usize {
        return Err(Error::TooManyImages);
    }

    for (ct, _, _) in &req.images {
        upload::validate_content_type(ct)?;
    }

    let id = storage::next_post_id();
    let caller = ic_cdk::caller();
    let now = ic_cdk::api::time();

    let images: Vec<PostImage> = req
        .images
        .into_iter()
        .map(|(content_type, size, chunk_count)| PostImage {
            content_type,
            size,
            chunk_count,
            width: None,
            height: None,
        })
        .collect();

    let status = if images.is_empty() {
        PostStatus::Published
    } else {
        PostStatus::Pending {
            uploaded_images: vec![],
        }
    };

    let post = Post {
        id,
        author: caller,
        content: req.content,
        images,
        status,
        like_count: 0,
        comment_count: 0,
        created_at: now,
    };

    POSTS.with(|p| p.borrow_mut().insert(id, post));
    Ok(id)
}

#[update]
fn upload_post_image(
    post_id: u64,
    image_index: u32,
    chunk_index: u32,
    data: Vec<u8>,
) -> Result<(), Error> {
    auth::require_registered()?;

    POSTS.with(|p| {
        let posts = p.borrow();
        let post = posts.get(&post_id).ok_or(Error::PostNotFound)?;
        if post.author != ic_cdk::caller() {
            return Err(Error::NotAuthor);
        }
        Ok(())
    })?;

    upload::upload_image_chunk(post_id, image_index, chunk_index, data)
}

#[update]
fn finalize_post(post_id: u64) -> Result<(), Error> {
    auth::require_registered()?;

    POSTS.with(|p| {
        let posts = p.borrow();
        let post = posts.get(&post_id).ok_or(Error::PostNotFound)?;
        if post.author != ic_cdk::caller() {
            return Err(Error::NotAuthor);
        }
        Ok(())
    })?;

    upload::finalize_post_images(post_id)
}

#[update]
fn delete_post(post_id: u64) -> Result<(), Error> {
    let caller = ic_cdk::caller();

    let post = POSTS.with(|p| p.borrow().get(&post_id).ok_or(Error::PostNotFound))?;

    if post.author != caller {
        auth::require_admin()?;
    }

    upload::delete_post_images(post_id, &post.images);

    LIKES.with(|l| {
        let mut likes = l.borrow_mut();
        let keys: Vec<LikeKey> = likes
            .range(
                LikeKey {
                    post_id,
                    user: Principal::from_slice(&[]),
                }..,
            )
            .take_while(|(k, _)| k.post_id == post_id)
            .map(|(k, _)| k)
            .collect();
        for key in keys {
            likes.remove(&key);
        }
    });

    let comment_ids: Vec<u64> = POST_COMMENTS.with(|pc| {
        let pc = pc.borrow();
        let start = PostCommentKey {
            post_id,
            comment_id: 0,
        };
        let end = PostCommentKey {
            post_id,
            comment_id: u64::MAX,
        };
        pc.range(start..=end).map(|(k, _)| k.comment_id).collect()
    });

    COMMENTS.with(|c| {
        let mut comments = c.borrow_mut();
        for cid in &comment_ids {
            comments.remove(cid);
        }
    });

    POST_COMMENTS.with(|pc| {
        let mut pc = pc.borrow_mut();
        for cid in &comment_ids {
            pc.remove(&PostCommentKey {
                post_id,
                comment_id: *cid,
            });
        }
    });

    POSTS.with(|p| p.borrow_mut().remove(&post_id));

    Ok(())
}

#[query]
fn list_posts(offset: u64, limit: u64) -> PostsPage {
    let total = POSTS.with(|p| p.borrow().len());

    let posts: Vec<PostSummary> = POSTS.with(|p| {
        p.borrow()
            .iter()
            .rev()
            .filter(|(_, post)| matches!(post.status, PostStatus::Published))
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, post)| PostSummary {
                id: post.id,
                author: post.author,
                content: post.content.clone(),
                image_count: post.images.len() as u32,
                like_count: post.like_count,
                comment_count: post.comment_count,
                created_at: post.created_at,
            })
            .collect()
    });

    PostsPage { posts, total }
}

#[query]
fn get_post(post_id: u64) -> Result<Post, Error> {
    POSTS.with(|p| p.borrow().get(&post_id).ok_or(Error::PostNotFound))
}

#[update]
fn like_post(post_id: u64) -> Result<(), Error> {
    auth::require_registered()?;
    let caller = ic_cdk::caller();

    POSTS.with(|p| {
        if p.borrow().get(&post_id).is_none() {
            return Err(Error::PostNotFound);
        }
        Ok(())
    })?;

    let key = LikeKey {
        post_id,
        user: caller,
    };

    LIKES.with(|l| {
        if l.borrow().contains_key(&key) {
            return Err(Error::AlreadyLiked);
        }
        l.borrow_mut().insert(key, ());
        Ok(())
    })?;

    POSTS.with(|p| {
        let mut posts = p.borrow_mut();
        if let Some(mut post) = posts.get(&post_id) {
            post.like_count += 1;
            posts.insert(post_id, post);
        }
    });

    Ok(())
}

#[update]
fn unlike_post(post_id: u64) -> Result<(), Error> {
    auth::require_registered()?;
    let caller = ic_cdk::caller();

    let key = LikeKey {
        post_id,
        user: caller,
    };

    LIKES.with(|l| {
        if l.borrow_mut().remove(&key).is_none() {
            return Err(Error::NotLiked);
        }
        Ok(())
    })?;

    POSTS.with(|p| {
        let mut posts = p.borrow_mut();
        if let Some(mut post) = posts.get(&post_id) {
            post.like_count = post.like_count.saturating_sub(1);
            posts.insert(post_id, post);
        }
    });

    Ok(())
}

#[query]
fn is_liked(post_id: u64) -> bool {
    let caller = ic_cdk::caller();
    let key = LikeKey {
        post_id,
        user: caller,
    };
    LIKES.with(|l| l.borrow().contains_key(&key))
}

#[update]
fn add_comment(
    post_id: u64,
    content: String,
    reply_to_comment_id: Option<u64>,
) -> Result<u64, Error> {
    auth::require_registered()?;

    if content.len() > MAX_COMMENT_LENGTH {
        return Err(Error::ContentTooLong);
    }

    POSTS.with(|p| {
        if p.borrow().get(&post_id).is_none() {
            return Err(Error::PostNotFound);
        }
        Ok(())
    })?;

    let mut reply_to_user = None;
    let mut parent_comment_id = None;

    if let Some(parent_id) = reply_to_comment_id {
        let parent =
            COMMENTS.with(|c| c.borrow().get(&parent_id).ok_or(Error::InvalidReplyTarget))?;

        if parent.reply_to_comment_id.is_some() {
            return Err(Error::ReplyToReply);
        }

        if parent.post_id != post_id {
            return Err(Error::CommentPostMismatch);
        }

        reply_to_user = Some(parent.author);
        parent_comment_id = Some(parent_id);
    }

    let id = storage::next_comment_id();
    let caller = ic_cdk::caller();
    let now = ic_cdk::api::time();

    let comment = Comment {
        id,
        post_id,
        author: caller,
        content,
        created_at: now,
        reply_to_comment_id: parent_comment_id,
        reply_to_user,
        reply_count: 0,
    };

    COMMENTS.with(|c| c.borrow_mut().insert(id, comment));
    POST_COMMENTS.with(|pc| {
        pc.borrow_mut().insert(
            PostCommentKey {
                post_id,
                comment_id: id,
            },
            (),
        );
    });

    if let Some(parent_id) = parent_comment_id {
        COMMENTS.with(|c| {
            let mut comments = c.borrow_mut();
            if let Some(mut parent) = comments.get(&parent_id) {
                parent.reply_count += 1;
                comments.insert(parent_id, parent);
            }
        });

        COMMENT_REPLIES.with(|cr| {
            cr.borrow_mut().insert(
                CommentReplyKey {
                    comment_id: parent_id,
                    reply_id: id,
                },
                (),
            );
        });
    }

    POSTS.with(|p| {
        let mut posts = p.borrow_mut();
        if let Some(mut post) = posts.get(&post_id) {
            post.comment_count += 1;
            posts.insert(post_id, post);
        }
    });

    Ok(id)
}

#[update]
fn delete_comment(comment_id: u64) -> Result<(), Error> {
    let caller = ic_cdk::caller();

    let comment = COMMENTS.with(|c| c.borrow().get(&comment_id).ok_or(Error::CommentNotFound))?;

    if comment.author != caller {
        auth::require_admin()?;
    }

    let total_deletes = if comment.reply_to_comment_id.is_none() {
        let reply_ids: Vec<u64> = COMMENT_REPLIES.with(|cr| {
            let cr = cr.borrow();
            let start = CommentReplyKey {
                comment_id,
                reply_id: 0,
            };
            let end = CommentReplyKey {
                comment_id,
                reply_id: u64::MAX,
            };
            cr.range(start..=end).map(|(k, _)| k.reply_id).collect()
        });

        COMMENTS.with(|c| {
            let mut comments = c.borrow_mut();
            for reply_id in &reply_ids {
                comments.remove(reply_id);
            }
        });

        POST_COMMENTS.with(|pc| {
            let mut pc = pc.borrow_mut();
            for reply_id in &reply_ids {
                pc.remove(&PostCommentKey {
                    post_id: comment.post_id,
                    comment_id: *reply_id,
                });
            }
        });

        COMMENT_REPLIES.with(|cr| {
            let mut cr = cr.borrow_mut();
            for reply_id in &reply_ids {
                cr.remove(&CommentReplyKey {
                    comment_id,
                    reply_id: *reply_id,
                });
            }
        });

        1 + reply_ids.len() as u64
    } else {
        if let Some(parent_id) = comment.reply_to_comment_id {
            COMMENTS.with(|c| {
                let mut comments = c.borrow_mut();
                if let Some(mut parent) = comments.get(&parent_id) {
                    parent.reply_count = parent.reply_count.saturating_sub(1);
                    comments.insert(parent_id, parent);
                }
            });

            COMMENT_REPLIES.with(|cr| {
                cr.borrow_mut().remove(&CommentReplyKey {
                    comment_id: parent_id,
                    reply_id: comment_id,
                });
            });
        }

        1
    };

    COMMENTS.with(|c| c.borrow_mut().remove(&comment_id));
    POST_COMMENTS.with(|pc| {
        pc.borrow_mut().remove(&PostCommentKey {
            post_id: comment.post_id,
            comment_id,
        });
    });

    POSTS.with(|p| {
        let mut posts = p.borrow_mut();
        if let Some(mut post) = posts.get(&comment.post_id) {
            post.comment_count = post.comment_count.saturating_sub(total_deletes);
            posts.insert(comment.post_id, post);
        }
    });

    Ok(())
}

#[derive(CandidType)]
struct CommentsPage {
    comments: Vec<Comment>,
    total: u64,
}

#[query]
fn list_comments(post_id: u64, offset: u64, limit: u64) -> CommentsPage {
    let comment_ids: Vec<u64> = POST_COMMENTS.with(|pc| {
        let pc = pc.borrow();
        let start = PostCommentKey {
            post_id,
            comment_id: 0,
        };
        let end = PostCommentKey {
            post_id,
            comment_id: u64::MAX,
        };
        pc.range(start..=end).map(|(k, _)| k.comment_id).collect()
    });

    let total = comment_ids.len() as u64;

    let comments: Vec<Comment> = COMMENTS.with(|c| {
        let comments = c.borrow();
        comment_ids
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .filter_map(|id| comments.get(id))
            .collect()
    });

    CommentsPage { comments, total }
}

#[query]
fn list_replies(comment_id: u64, offset: u64, limit: u64) -> CommentsPage {
    let reply_ids: Vec<u64> = COMMENT_REPLIES.with(|cr| {
        let cr = cr.borrow();
        let start = CommentReplyKey {
            comment_id,
            reply_id: 0,
        };
        let end = CommentReplyKey {
            comment_id,
            reply_id: u64::MAX,
        };
        cr.range(start..=end).map(|(k, _)| k.reply_id).collect()
    });

    let total = reply_ids.len() as u64;

    let comments: Vec<Comment> = COMMENTS.with(|c| {
        let comments = c.borrow();
        reply_ids
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .filter_map(|id| comments.get(id))
            .collect()
    });

    CommentsPage { comments, total }
}

#[query]
fn http_request(req: HttpRequest) -> HttpResponse {
    let path = req.url.split('?').next().unwrap_or(&req.url);

    if let Some(rest) = path.strip_prefix("/post/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() == 3 && parts[1] == "image" {
            if let (Ok(post_id), Ok(image_index)) =
                (parts[0].parse::<u64>(), parts[2].parse::<u32>())
            {
                return serve_image(post_id, image_index);
            }
        }
    }

    HttpResponse {
        status_code: 404,
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: b"Not Found. Use /post/{id}/image/{index}".to_vec(),
    }
}

fn serve_image(post_id: u64, image_index: u32) -> HttpResponse {
    match upload::get_image_data(post_id, image_index) {
        Ok((content_type, data)) => HttpResponse {
            status_code: 200,
            headers: vec![
                ("Content-Type".to_string(), content_type),
                (
                    "Cache-Control".to_string(),
                    "public, max-age=604800".to_string(),
                ),
            ],
            body: data,
        },
        Err(_) => HttpResponse {
            status_code: 404,
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: b"Image not found".to_vec(),
        },
    }
}

ic_cdk::export_candid!();
