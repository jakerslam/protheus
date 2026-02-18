#!/usr/bin/env node
const fs = require('fs');
const data = fs.readFileSync(0, 'utf8');
const json = JSON.parse(data);
if (json.success && json.posts) {
  json.posts.slice(0, 5).forEach((p, i) => {
    console.log(`${i+1}. [${p.upvotes}] ${p.title} by @${p.author.name} (id: ${p.id})`);
  });
}