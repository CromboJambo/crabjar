---
name: session-persistence
description: |
  Implement session persistence for auth state management. Use whenever the user wants to auto-save/restore cookies and localStorage by name — session persistence enables auth state management across browser sessions. Trigger when the user mentions session persistence, cookie save, localStorage restore, auth state management, or wants to build auth state persistence tooling.
---

# Session Persistence

## Core Concept

Session persistence auto-save/restore cookies and localStorage by name. This enables auth state management across browser sessions — each session has its own browser instance, cookies, storage, navigation history, authentication state.

## Session Model

| Session Property | Description |
|---|---|
| **Name** | Unique session identifier |
| **Browser** | Isolated browser instance |
| **Cookies** | Session-specific cookies |
| **Storage** | Session-specific localStorage/sessionStorage |
| **Navigation** | Session-specific navigation history |
| **Auth** | Session-specific authentication state |

## Implementation

### 1. Session Creation

Create isolated session:
- **Session name**: unique identifier
- **Browser instance**: new browser process
- **Profile**: session-specific browser profile
- **Storage**: session-specific cookie/storage store

### 2. Cookie Save

Save session cookies:
- **Format**: serialized cookie array
- **Encryption**: AES-256-GCM optional
- **Storage**: session-specific file
- **Name**: session name as key

### 3. Cookie Restore

Restore session cookies:
- **Load**: session-specific cookie file
- **Inject**: cookies into browser instance
- **Verify**: cookie validity check
- **Error**: surface on restore failure

### 4. Storage Save

Save session storage:
- **localStorage**: serialized localStorage data
- **sessionStorage**: serialized sessionStorage data
- **Encryption**: AES-256-GCM optional
- **Storage**: session-specific file

### 5. Storage Restore

Restore session storage:
- **Load**: session-specific storage file
- **Inject**: storage into browser instance
- **Verify**: storage validity check
- **Error**: surface on restore failure

## Configuration

- **Session name**: manual, auto-generated, env var
- **Encryption**: AES-256-GCM, disabled, optional
- **Auto-save**: on session end, on idle timeout, manual
- **Auto-restore**: on session start, on reconnect, manual
- **Storage location**: session-specific file, shared, encrypted

## Integration with ${PROJECT}

${PROJECT}'s auth state management should adopt:
- Session persistence for cookie/localStorage save/restore
- AES-256-GCM encryption for sensitive data
- Session-specific storage isolation
- Auto-save/restore on idle timeout
- Encryption key management
