# Mobile Development — iOS & Android

## SwiftUI (iOS)

### Component Generation Rules
- Use `@State` for local view state, `@Binding` for parent-owned state
- `@Observable` (iOS 17+) over `@ObservableObject` for model classes
- Prefer `NavigationStack` over deprecated `NavigationView`
- Use `.task { }` modifier for async data loading (auto-cancelled on disappear)
- Include Preview macros for every view component

### SwiftUI Patterns
```swift
struct UserList: View {
    @State private var users: [User] = []
    @State private var isLoading = true

    var body: some View {
        NavigationStack {
            List(users) { user in
                NavigationLink(value: user) { UserRow(user: user) }
            }
            .navigationTitle("Users")
            .overlay { if isLoading { ProgressView() } }
            .task { users = await fetchUsers(); isLoading = false }
            .navigationDestination(for: User.self) { UserDetail(user: $0) }
        }
    }
}
```

### On-Device AI (iOS 26 Foundation Models)
```swift
import FoundationModels

let session = LanguageModelSession()
let response = try await session.respond(to: "Summarize this text: ...")

// Structured output with @Generable
@Generable
struct Sentiment {
    let label: String  // "positive", "negative", "neutral"
    let confidence: Double
}
let result: Sentiment = try await session.respond(to: prompt)
```

## Jetpack Compose (Android)

### Component Generation Rules
- State hoisting: UI components accept state + event callbacks
- Use `remember` + `mutableStateOf` for local state
- `collectAsStateWithLifecycle()` for Flow observation
- `LazyColumn`/`LazyRow` for lists (never `Column` with `forEach`)
- Include `@Preview` for every composable

### Compose Patterns
```kotlin
@Composable
fun UserList(
    users: List<User>,
    onUserClick: (User) -> Unit,
    modifier: Modifier = Modifier
) {
    LazyColumn(modifier) {
        items(users, key = { it.id }) { user ->
            UserRow(user = user, onClick = { onUserClick(user) })
        }
    }
}

// ViewModel
class UserViewModel : ViewModel() {
    private val _users = MutableStateFlow<List<User>>(emptyList())
    val users = _users.asStateFlow()

    init { viewModelScope.launch { _users.value = repository.getUsers() } }
}
```

## Cross-Platform (React Native / Flutter / Compose Multiplatform)

### DESIGN.md System (Pixel-Accurate Generation)
Create a `DESIGN.md` at project root with:
```markdown
## Colors
- primary: #007AFF (iOS) / #6200EE (Material)
- background: #FFFFFF (light) / #000000 (dark)

## Typography
- heading: SF Pro Display 28pt bold / Roboto 28sp bold
- body: SF Pro Text 17pt / Roboto 16sp

## Spacing
- xs: 4, sm: 8, md: 16, lg: 24, xl: 32

## Components
- Button: height 44pt (iOS) / 48dp (Android), corner radius 8
  - States: default, pressed (opacity 0.7), disabled (opacity 0.4)
```

### Flutter Rules
- `StatelessWidget` default; `StatefulWidget` only when local state needed
- Use `const` constructors wherever possible
- `ListView.builder` for dynamic lists (not `ListView(children:)`)
- Prefer `Provider` / `Riverpod` for state management
- `ThemeData` for consistent styling; never hardcode colors

### React Native Rules
- `FlatList` for lists (not `ScrollView` with map)
- `StyleSheet.create()` for styles (not inline objects)
- Use `react-native-reanimated` for animations (not `Animated`)
- Navigation: `@react-navigation/native` with typed routes

## Mobile Testing

### GPTDroid Testing Pattern (+32% coverage, +31% bugs)
```
1. Capture GUI state (view hierarchy, widget types, text content)
2. LLM analyzes: "What action should a tester take to explore functionality?"
3. Execute suggested action (tap, scroll, type, navigate)
4. Capture resulting screen state + any crashes/errors
5. Feed back: "What changed? Expected? What to test next?"
6. Maintain memory of visited screens to guide exploration
7. Run until all reachable screens explored or time budget exhausted
```

### Platform-Specific Test Commands
```bash
# iOS
xcodebuild test -scheme MyApp -destination 'platform=iOS Simulator,name=iPhone 16'

# Android
./gradlew connectedAndroidTest

# Flutter
flutter test
flutter drive --driver=test_driver/integration_test.dart

# React Native
npx jest --coverage
npx detox test --configuration ios.sim.release
```

## Mobile Performance Checklist

- App startup: < 1s cold start (measure with Instruments/Android Profiler)
- List scrolling: 60fps (no dropped frames in scroll traces)
- Memory: no unbounded growth (watch for retain cycles / leaked listeners)
- Battery: no background CPU usage when idle
- Network: cache aggressively, use pagination, compress payloads
- Images: use platform-native lazy loading, resize server-side
