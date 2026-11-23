# API Request Panel - Component Structure

## Visual Layout

```
┌─────────────────────────────────────────────────────────────┐
│ API Request Panel                                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ [POST] Endpoint                                    [📋] │ │
│ ├─────────────────────────────────────────────────────────┤ │
│ │ https://api.anchorkit.stellar.org/v1/attestations      │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Request Body                                       [📋] │ │
│ ├─────────────────────────────────────────────────────────┤ │
│ │ {                                                       │ │
│ │   "issuer": "GANCHOR123...",                           │ │
│ │   "subject": "GUSER456...",                            │ │
│ │   "timestamp": 1708819200                              │ │
│ │ }                                                       │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Response                                           [📋] │ │
│ ├─────────────────────────────────────────────────────────┤ │
│ │ {                                                       │ │
│ │   "success": true,                                     │ │
│ │   "attestation_id": "att_123456",                      │ │
│ │   "status": "confirmed"                                │ │
│ │ }                                                       │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ cURL Command                                       [📋] │ │
│ ├─────────────────────────────────────────────────────────┤ │
│ │ curl -X POST \                                         │ │
│ │   "https://api.anchorkit.stellar.org/v1/attestations" │ │
│ │   -H "Content-Type: application/json" \                │ │
│ │   -d '{"issuer":"GANCHOR123..."}'                      │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Component Hierarchy

```
ApiRequestPanel
├── EndpointSection
│   ├── SectionHeader
│   │   ├── MethodBadge (GET/POST/PUT/DELETE/PATCH)
│   │   └── Title ("Endpoint")
│   └── SectionContent
│       ├── EndpointURL (code element)
│       └── CopyButton
│
├── RequestSection (conditional)
│   ├── SectionHeader
│   │   ├── Title ("Request Body")
│   │   └── CopyButton
│   └── SectionContent
│       └── CodeBlock (formatted JSON)
│
├── ResponseSection
│   ├── SectionHeader
│   │   ├── Title ("Response")
│   │   └── CopyButton (conditional)
│   └── SectionContent
│       ├── SkeletonLoader (if loading)
│       ├── ErrorMessage (if error)
│       ├── CodeBlock (if response)
│       └── EmptyState (if no response)
│
└── CurlSection
    ├── SectionHeader
    │   ├── Title ("cURL Command")
    │   └── CopyButton
    └── SectionContent
        └── CodeBlock (generated cURL)
```

## State Flow

```
┌──────────────┐
│ Initial Load │
└──────┬───────┘
       │
       ▼
┌──────────────────┐
│ Empty State      │
│ - No response    │
│ - Show endpoint  │
│ - Show request   │
└──────┬───────────┘
       │
       │ User triggers API call
       ▼
┌──────────────────┐
│ Loading State    │
│ - Skeleton       │
│ - Animated       │
└──────┬───────────┘
       │
       ├─── Success ───┐
       │               ▼
       │        ┌──────────────────┐
       │        │ Success State    │
       │        │ - Show response  │
       │        │ - Enable copy    │
       │        └──────────────────┘
       │
       └─── Error ─────┐
                       ▼
                ┌──────────────────┐
                │ Error State      │
                │ - Show error msg │
                │ - Warning icon   │
                └──────────────────┘
```

## Data Flow

```
Props Input
    │
    ├─── endpoint ────────────► EndpointSection
    │                              │
    ├─── method ──────────────────┤
    │                              │
    ├─── requestBody ─────────► RequestSection
    │                              │
    ├─── response ────────────► ResponseSection
    │                              │
    ├─── isLoading ───────────────┤
    │                              │
    ├─── error ───────────────────┤
    │                              │
    └─── headers ─────────────► CurlSection
                                   │
                                   ▼
                            generateCurl()
                                   │
                                   ▼
                            cURL Command Display
```

## Copy Functionality Flow

```
User clicks Copy Button
    │
    ▼
Identify Section
    │
    ├─── Endpoint ──────► Copy URL string
    │
    ├─── Request ───────► Format JSON → Copy
    │
    ├─── Response ──────► Format JSON → Copy
    │
    └─── cURL ──────────► Generate cURL → Copy
                              │
                              ▼
                    navigator.clipboard.writeText()
                              │
                              ▼
                    Show checkmark (✓) for 2s
                              │
                              ▼
                    Reset to clipboard icon (📋)
```

## Styling Architecture

```
api-request-panel (container)
    │
    ├─── panel-section (repeatable)
    │       │
    │       ├─── section-header
    │       │       │
    │       │       ├─── method-badge (conditional)
    │       │       ├─── h3 (title)
    │       │       └─── copy-button
    │       │
    │       └─── section-content
    │               │
    │               ├─── endpoint-url (code)
    │               ├─── code-block (pre > code)
    │               ├─── skeleton-loader
    │               ├─── error-message
    │               └─── empty-state
    │
    └─── Responsive breakpoints
            │
            ├─── Desktop (> 768px)
            │       └─── Horizontal layout
            │
            └─── Mobile (≤ 768px)
                    └─── Vertical stack
```

## Method Badge Colors

```
GET    → Blue   (#1e40af on #dbeafe)
POST   → Green  (#065f46 on #d1fae5)
PUT    → Yellow (#92400e on #fef3c7)
DELETE → Red    (#991b1b on #fee2e2)
PATCH  → Purple (#3730a3 on #e0e7ff)
```

## Responsive Breakpoints

```
Desktop (> 768px)
├─── Full width sections
├─── Horizontal button layout
└─── Side-by-side elements

Mobile (≤ 768px)
├─── Stacked sections
├─── Full-width buttons
└─── Vertical layout
```

## Dark Mode Mapping

```
Light Mode              →  Dark Mode
─────────────────────────────────────
#ffffff (white)         →  #1f2937
#fafafa (light gray)    →  #111827
#f3f4f6 (lighter gray)  →  #1f2937
#1f2937 (dark)          →  #0f172a
#e5e7eb (border)        →  #374151
#1f2937 (text dark)     →  #f9fafb
```

## Event Handlers

```
Component Events
    │
    ├─── onClick (Copy Button)
    │       │
    │       └─── copyToClipboard(text, section)
    │               │
    │               ├─── navigator.clipboard.writeText()
    │               ├─── setCopiedSection(section)
    │               └─── setTimeout(() => reset, 2000)
    │
    └─── useEffect (Auto-reset copied state)
            │
            └─── Clear after 2 seconds
```

## File Dependencies

```
ApiRequestPanel.tsx
    │
    ├─── imports React, useState
    │
    ├─── imports ./ApiRequestPanel.css
    │
    └─── exports ApiRequestPanel, ApiRequestPanelProps

ApiRequestPanel.css
    │
    ├─── Base styles
    ├─── Component styles
    ├─── Responsive styles (@media)
    └─── Dark mode styles (@media prefers-color-scheme)

ApiRequestPanel.test.tsx
    │
    ├─── imports @testing-library/react
    ├─── imports @testing-library/jest-dom
    └─── imports ApiRequestPanel

ApiRequestPanel.example.tsx
    │
    ├─── imports ApiRequestPanel
    └─── exports usage examples
```

## Integration Points

```
AnchorKit Application
    │
    ├─── Contract Calls
    │       │
    │       └─── ApiRequestPanel
    │               │
    │               ├─── Display request
    │               ├─── Show loading
    │               └─── Display response
    │
    ├─── Skeleton Loaders
    │       │
    │       └─── ApiRequestPanel (isLoading prop)
    │
    └─── Session Tracking
            │
            └─── ApiRequestPanel (headers with session ID)
```

## Performance Considerations

```
Optimization Strategy
    │
    ├─── Minimal Re-renders
    │       └─── useState for local state only
    │
    ├─── Efficient Clipboard
    │       └─── Native navigator.clipboard API
    │
    ├─── CSS Animations
    │       └─── GPU-accelerated transforms
    │
    └─── Code Splitting
            └─── Lazy load if needed
```

## Accessibility Tree

```
div[role="region"] (api-request-panel)
    │
    ├─── div (endpoint-section)
    │       ├─── h3 "Endpoint"
    │       ├─── code (endpoint URL)
    │       └─── button[title="Copy endpoint"]
    │
    ├─── div (request-section)
    │       ├─── h3 "Request Body"
    │       ├─── pre > code (JSON)
    │       └─── button[title="Copy request"]
    │
    ├─── div (response-section)
    │       ├─── h3 "Response"
    │       ├─── pre > code (JSON) | div[role="status"] (loading/error)
    │       └─── button[title="Copy response"]
    │
    └─── div (curl-section)
            ├─── h3 "cURL Command"
            ├─── pre > code (cURL)
            └─── button[title="Copy cURL"]
```

---

This structure ensures:
- ✅ Clear component hierarchy
- ✅ Predictable state flow
- ✅ Efficient rendering
- ✅ Accessible markup
- ✅ Maintainable code
