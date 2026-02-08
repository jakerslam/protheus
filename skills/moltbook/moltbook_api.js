// skills/moltbook/moltbook_api.js
// Core Moltbook functions for agent automation

const apiBase = 'https://www.moltbook.com/api/v1';
const getAuthHeader = (apiKey) => ({ 'Authorization': 'Bearer ' + apiKey });

async function moltbook_getHotPosts(limit = 5, apiKey) {
  const res = await fetch(`${apiBase}/posts?sort=hot&limit=${limit}`, {
    headers: getAuthHeader(apiKey)
  });
  return await res.json();
}

async function moltbook_upvotePost(postId, apiKey) {
  const res = await fetch(`${apiBase}/posts/${postId}/upvote`, {
    method: 'POST',
    headers: getAuthHeader(apiKey)
  });
  return await res.json();
}

async function moltbook_comment(postId, text, apiKey) {
  const res = await fetch(`${apiBase}/posts/${postId}/comments`, {
    method: 'POST',
    headers: { ...getAuthHeader(apiKey), 'Content-Type': 'application/json' },
    body: JSON.stringify({ text })
  });
  return await res.json();
}

async function moltbook_createPost(title, body, apiKey) {
  const res = await fetch(`${apiBase}/posts`, {
    method: 'POST',
    headers: { ...getAuthHeader(apiKey), 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, body })
  });
  return await res.json();
}

async function moltbook_listAgents(limit = 10, apiKey) {
  const res = await fetch(`${apiBase}/agents?sort=active&limit=${limit}`, {
    headers: getAuthHeader(apiKey)
  });
  return await res.json();
}

module.exports = {
  moltbook_getHotPosts,
  moltbook_upvotePost,
  moltbook_comment,
  moltbook_createPost,
  moltbook_listAgents
};
