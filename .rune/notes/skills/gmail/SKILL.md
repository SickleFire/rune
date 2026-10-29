---
name: gmail
description: Guides Rune in designing, implementing, authenticating, and interacting with the Gmail API (REST, OAuth 2.0, messages, threads, drafts, labels, and push notifications) for automation and email management skills. Use whenever tasked with creating Gmail API clients, sending/reading emails via Gmail REST API, handling OAuth credentials, or building email workflow automation.
compatibility: Requires Python 3.x / Node.js / Go / Java (depending on client implementation), Google Cloud project credentials, and Gmail API enabled.
---

# Gmail API Integration Skill

This skill enables Rune to assist with designing, developing, testing, and debugging integrations with the **Gmail API** (`https://developers.google.com/workspace/gmail/api/guides`).

---

## Core Capabilities & Resources Covered
1. **Authentication & Authorization**: OAuth 2.0 flow, setting up scopes (`https://www.googleapis.com/auth/gmail.readonly`, `https://www.googleapis.com/auth/gmail.send`, `https://www.googleapis.com/auth/gmail.modify`, `https://mail.google.com/`), and token management.
2. **Mail Management**:
   - **Messages**: Create, send, read, list, search, filter (`q` parameter like `from:user@example.com is:unread`), and delete messages or upload attachments.
   - **Threads**: Manage conversation threads and replies.
   - **Drafts**: Create, update, and send unsent draft messages.
   - **Labels**: Apply, remove, create, and modify system and user labels (e.g., `INBOX`, `SPAM`, custom labels).
3. **Advanced Features**:
   - Push notifications via Google Cloud Pub/Sub (`users.watch`).
   - Batch requests for performance optimization.
   - Error handling and exponential backoff for rate limits.

---

## Workflow for Implementing Gmail API Integration

### Step 1: Define Use Case & Scopes
Determine what permissions and operations are required:
- **Read-only**: Extraction, indexing, backup -> Scope: `https://www.googleapis.com/auth/gmail.readonly`
- **Sending mail**: Automated alerts, notifications -> Scope: `https://www.googleapis.com/auth/gmail.send`
- **Full modification**: Filtering, labeling, sorting -> Scope: `https://www.googleapis.com/auth/gmail.modify` or `https://mail.google.com/`

### Step 2: Set Up Google Cloud Project & Credentials
1. Go to the [Google Cloud Console](https://console.cloud.google.com/).
2. Create or select a project.
3. Enable the **Gmail API**.
4. Configure the **OAuth consent screen** (Internal or External).
5. Create **OAuth 2.0 Client IDs** (Desktop app, Web application, or Service Account with domain-wide delegation if applicable).
6. Download `credentials.json` or configure environment variables (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REFRESH_TOKEN`).

### Step 3: Implement API Client (Python Reference Example)
When implementing a Python integration using `google-api-python-client` and `google-auth`:

```python
import os
import base64
from email.message import EmailMessage
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError

# If modifying these scopes, delete the file token.json.
SCOPES = ['https://www.googleapis.com/auth/gmail.readonly', 'https://www.googleapis.com/auth/gmail.send']

def get_gmail_service():
    creds = None
    # The file token.json stores the user's access and refresh tokens.
    if os.path.exists('token.json'):
        creds = Credentials.from_authorized_user_file('token.json', SCOPES)
    # If there are no (valid) credentials available, let the user log in.
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            creds.refresh(Request())
        else:
            flow = InstalledAppFlow.from_client_secrets_file(
                'credentials.json', SCOPES)
            creds = flow.run_local_server(port=0)
        # Save the credentials for the next run
        with open('token.json', 'w') as token:
            token.write(creds.to_json())

    return build('gmail', 'v1', credentials=creds)

def list_messages(service, user_id='me', query='is:unread'):
    try:
        response = service.users().messages().list(userId=user_id, q=query).execute()
        messages = response.get('messages', [])
        return messages
    except HttpError as error:
        print(f'An error occurred: {error}')
        return []

def send_message(service, sender, to, subject, body_text, user_id='me'):
    try:
        message = EmailMessage()
        message.set_content(body_text)
        message['To'] = to
        message['From'] = sender
        message['Subject'] = subject

        # Encode message
        encoded_message = base64.urlsafe_b64encode(message.as_bytes()).decode()
        create_message = {'raw': encoded_message}
        
        send_message = (service.users().messages().send(userId=user_id, body=create_message).execute())
        print(f'Message Id: {send_message["id"]}')
        return send_message
    except HttpError as error:
        print(f'An error occurred: {error}')
        return None
```

### Step 4: Validate and Test
- Verify OAuth authentication token generation.
- Test query filters (e.g. `subject:"Invoice"`, `has:attachment`).
- Test batch operations and error handling (handling HTTP 429 Rate Limit Exceeded with exponential backoff).
