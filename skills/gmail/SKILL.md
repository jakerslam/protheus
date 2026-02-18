---
name: gmail
description: Send, read, and manage Gmail emails via the Gmail API. Use when the user needs to (1) Send emails or replies, (2) Check inbox or search for messages, (3) Read email content and attachments, (4) Manage labels/folders, (5) Work with drafts, or any other Gmail-related tasks requiring programmatic email access.
---

# Gmail Integration

Enable programmatic access to Gmail for sending, reading, and managing emails through the Gmail API.

## Setup

### 1. Create Google Cloud Project & OAuth Credentials

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Enable the **Gmail API** (APIs & Services → Library → Gmail API → Enable)
4. Create OAuth 2.0 credentials:
   - APIs & Services → Credentials → Create Credentials → OAuth client ID
   - Application type: Desktop app
   - Download the `client_secret.json` file
5. Add `credentials.json` to the skill directory

### 2. First Authentication (Token Generation)

Run once to authenticate and generate `token.json`:

```bash
python3 scripts/gmail_auth.py
```

This opens a browser for OAuth consent. After approval, `token.json` is saved for future API calls.

**Note:** If token expires or scopes change, delete `token.json` and re-run auth.

## Quick Start

**Send an email:**
```bash
python3 scripts/gmail_send.py --to "recipient@example.com" --subject "Hello" --body "Message text" [--html]
```

**Search inbox:**
```bash
python3 scripts/gmail_search.py --query "from:sender@example.com newer_than:2d" [--limit 10]
```

**Read a message:**
```bash
python3 scripts/gmail_read.py --id "MESSAGE_ID"
```

**List labels:**
```bash
python3 scripts/gmail_labels.py
```

## Core Capabilities

### 1. Sending Emails

- Plain text or HTML messages
- CC/BCC support
- Attachments
- Reply threading (References/In-Reply-To headers)

### 2. Reading & Search

- Full-text search with Gmail query syntax
- Filter by: from, to, subject, has:attachment, newer_than:, etc.
- Retrieve message body (plain text/HTML)
- Download attachments
- Thread-based conversations

### 3. Managing Messages

- Apply/remove labels
- Mark read/unread
- Archive/delete
- Move between folders

### 4. Drafts

- Create drafts
- Update existing drafts
- Send drafts

## Gmail Query Syntax Reference

Common search operators:
- `from:email@domain.com` - From specific sender
- `to:email@domain.com` - To specific recipient
- `subject:keyword` - Subject contains keyword
- `has:attachment` - Has attachments
- `newer_than:2d` - Newer than 2 days (d=days, w=weeks, m=months)
- `older_than:1w` - Older than 1 week
- `is:unread` - Unread messages
- `is:starred` - Starred messages
- `in:inbox` - In inbox
- `in:sent` - Sent messages
- `in:trash` - Deleted messages
- `label:labelname` - With specific label
- `filename:pdf` - Attachment type
- `larger:5M` - Larger than 5MB

## Scripts Reference

### scripts/gmail_auth.py
Initial OAuth flow to generate `token.json`.

### scripts/gmail_send.py
Send emails with support for HTML, attachments, Cc/Bcc.

### scripts/gmail_search.py
Search messages with query syntax.

### scripts/gmail_read.py
Read message content and download attachments.

### scripts/gmail_labels.py
List, create, or modify labels.

### scripts/gmail_modify.py
Modify messages (label, mark read/unread, archive).

## API Reference

For detailed API documentation, see [references/gmail_api.md](references/gmail_api.md).

## Security Notes

- Store `credentials.json` and `token.json` securely
- Never commit these files to version control
- Use restricted OAuth scopes (prefer `gmail.send` + `gmail.readonly` if read-only needed)
- Token refresh is automatic when using the provided scripts
