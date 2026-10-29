import os
import base64
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError

# Read-only scope for reading emails
SCOPES = ['https://www.googleapis.com/auth/gmail.readonly']

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

def read_emails(max_results=5, query=''):
    service = get_gmail_service()
    if not service:
        return

    try:
        print(f"Fetching messages (query: '{query}', max: {max_results})...\n")
        results = service.users().messages().list(userId='me', q=query, maxResults=max_results).execute()
        messages = results.get('messages', [])

        if not messages:
            print("No messages found.")
            return

        for msg_info in messages:
            msg = service.users().messages().get(userId='me', id=msg_info['id'], format='full').execute()
            
            headers = msg['payload'].get('headers', [])
            subject = next((h['value'] for h in headers if h['name'].lower() == 'subject'), '(No Subject)')
            sender = next((h['value'] for h in headers if h['name'].lower() == 'from'), '(Unknown Sender)')
            date = next((h['value'] for h in headers if h['name'].lower() == 'date'), '(Unknown Date)')
            snippet = msg.get('snippet', '')

            print(f"ID: {msg_info['id']}")
            print(f"Date: {date}")
            print(f"From: {sender}")
            print(f"Subject: {subject}")
            print(f"Snippet: {snippet}")
            print("-" * 60)

    except HttpError as error:
        print(f"An error occurred: {error}")

if __name__ == '__main__':
    read_emails()
