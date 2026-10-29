import os
import sys
import base64
from email.message import EmailMessage
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build
from googleapiclient.errors import HttpError

# Send scope for sending emails
SCOPES = ['https://www.googleapis.com/auth/gmail.send']

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

def send_email(to, subject, body):
    service = get_gmail_service()
    if not service:
        return

    try:
        message = EmailMessage()
        message.set_content(body)
        message['To'] = to
        message['From'] = 'me'
        message['Subject'] = subject

        raw_message = base64.urlsafe_b64encode(message.as_bytes()).decode('utf-8')
        create_message = {'raw': raw_message}
        
        sent_message = service.users().messages().send(userId='me', body=create_message).execute()
        print(f"Email sent successfully! Message ID: {sent_message['id']}")
    except HttpError as error:
        print(f"An error occurred: {error}")

if __name__ == '__main__':
    if len(sys.argv) < 4:
        print("Usage: python send_email.py <to> <subject> <body>")
        sys.exit(1)
    send_email(sys.argv[1], sys.argv[2], sys.argv[3])
