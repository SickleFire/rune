import os
import sys
import base64
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError

# Modify scope for managing labels and messages
SCOPES = ['https://www.googleapis.com/auth/gmail.modify']

def get_gmail_service():
    creds = None
    if os.path.exists('token.json'):
        creds = Credentials.from_authorized_user_file('token.json', SCOPES)
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            creds.refresh(Request())
        else:
            if not os.path.exists('credentials.json'):
                print("Error: 'credentials.json' not found. Please place your Google Cloud OAuth client credentials in 'credentials.json'.")
                return None
            flow = InstalledAppFlow.from_client_secrets_file(
                'credentials.json', SCOPES)
            creds = flow.run_local_server(port=0)
        with open('token.json', 'w') as token:
            token.write(creds.to_json())
    return build('gmail', 'v1', credentials=creds)

def list_labels():
    service = get_gmail_service()
    if not service:
        return
    try:
        results = service.users().labels().list(userId='me').execute()
        labels = results.get('labels', [])
        if not labels:
            print("No labels found.")
            return
        print("Labels:")
        for label in labels:
            print(f"- {label['name']} (ID: {label['id']}, Type: {label.get('type', 'user')})")
    except HttpError as error:
        print(f"An error occurred: {error}")

def modify_message_labels(msg_id, add_labels=None, remove_labels=None):
    service = get_gmail_service()
    if not service:
        return
    body = {
        'addLabelIds': add_labels or [],
        'removeLabelIds': remove_labels or []
    }
    try:
        updated_msg = service.users().messages().batchModify(
            userId='me',
            body={'ids': [msg_id], **body}
        ).execute()
        print(f"Successfully modified labels for message ID: {msg_id}")
    except HttpError as error:
        print(f"An error occurred: {error}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python manage_labels.py list")
        print("  python manage_labels.py modify <msg_id> --add <label_id> --remove <label_id>")
        sys.exit(1)

    command = sys.argv[1]
    if command == 'list':
        list_labels()
    elif command == 'modify' and len(sys.argv) > 2:
        msg_id = sys.argv[2]
        add_lbls = []
        rem_lbls = []
        i = 3
        while i < len(sys.argv):
            if sys.argv[i] == '--add' and i + 1 < len(sys.argv):
                add_lbls.append(sys.argv[i+1])
                i += 2
            elif sys.argv[i] == '--remove' and i + 1 < len(sys.argv):
                rem_lbls.append(sys.argv[i+1])
                i += 2
            else:
                i += 1
        modify_message_labels(msg_id, add_lbls, rem_lbls)
    else:
        print("Invalid command or arguments.")
