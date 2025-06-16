# Contributing to CGMiner-RS

Thank you for your interest in contributing to CGMiner-RS! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Git
- Linux development environment (Ubuntu 20.04+ recommended)
- Basic knowledge of Bitcoin mining and ASIC hardware

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/cgminer-rs.git
   cd cgminer-rs
   ```

2. **Set up Development Environment**
   ```bash
   make setup-dev
   ```

3. **Build and Test**
   ```bash
   make build
   make test
   ```

4. **Run Development Server**
   ```bash
   make dev
   ```

## Development Workflow

### Branch Naming

- `feature/description` - New features
- `bugfix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test improvements

### Commit Messages

Follow conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Build/tooling changes

Examples:
```
feat(device): add support for Antminer S19
fix(pool): resolve connection timeout issue
docs(api): update REST API documentation
```

### Pull Request Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clean, well-documented code
   - Add tests for new functionality
   - Update documentation as needed

3. **Test Your Changes**
   ```bash
   make test
   make lint
   make bench
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat(scope): description"
   ```

5. **Push and Create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create Pull Request**
   - Use the PR template
   - Provide clear description
   - Link related issues
   - Request review

## Code Standards

### Rust Style Guide

- Follow `rustfmt` formatting (run `make format`)
- Use `clippy` for linting (run `make lint`)
- Write comprehensive documentation
- Use meaningful variable and function names

### Documentation

- Document all public APIs
- Include examples in documentation
- Update README.md for significant changes
- Add inline comments for complex logic

### Testing

- Write unit tests for all new functions
- Add integration tests for new features
- Maintain test coverage above 80%
- Test error conditions and edge cases

### Performance

- Profile performance-critical code
- Use benchmarks for optimization
- Avoid unnecessary allocations
- Consider async/await patterns

## Project Structure

```
cgminer-rs/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Configuration management
│   ├── device/           # Device drivers and management
│   ├── mining/           # Mining logic and coordination
│   ├── pool/             # Pool management and Stratum
│   ├── api/              # REST API and WebSocket
│   ├── monitoring/       # System monitoring and alerts
│   ├── error.rs          # Error types and handling
│   └── ffi.rs            # C FFI bindings
├── drivers/              # Hardware driver implementations
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
├── docs/                 # Documentation
├── examples/             # Example configurations and scripts
└── build.rs              # Build script
```

## Adding New Features

### Device Drivers

To add support for new mining hardware:

1. **Create Driver Module**
   ```rust
   // src/device/your_device.rs
   use crate::device::{DeviceDriver, MiningDevice, DeviceInfo};
   
   pub struct YourDeviceDriver;
   
   impl DeviceDriver for YourDeviceDriver {
       // Implement required methods
   }
   ```

2. **Register Driver**
   ```rust
   // src/device/manager.rs
   device_manager.register_driver(Box::new(YourDeviceDriver::new()));
   ```

3. **Add Tests**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[tokio::test]
       async fn test_your_device_driver() {
           // Test implementation
       }
   }
   ```

### API Endpoints

To add new API endpoints:

1. **Define Handler**
   ```rust
   // src/api/handlers.rs
   pub async fn your_handler(
       State(state): State<AppState>,
   ) -> Result<Json<ApiResponse<YourResponse>>, ApiError> {
       // Implementation
   }
   ```

2. **Add Route**
   ```rust
   // src/api/mod.rs
   .route("/your-endpoint", get(your_handler))
   ```

3. **Update Documentation**
   ```markdown
   <!-- docs/API.md -->
   #### GET /your-endpoint
   Description of your endpoint
   ```

### Monitoring Metrics

To add new monitoring metrics:

1. **Define Metric Structure**
   ```rust
   // src/monitoring/mod.rs
   pub struct YourMetrics {
       pub timestamp: SystemTime,
       pub your_value: f64,
   }
   ```

2. **Implement Collection**
   ```rust
   // src/monitoring/metrics.rs
   async fn collect_your_metrics(&mut self) -> Result<YourMetrics, MiningError> {
       // Collection logic
   }
   ```

3. **Add to System**
   ```rust
   // src/monitoring/system.rs
   // Integrate into monitoring loop
   ```

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_name() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
    
    #[tokio::test]
    async fn test_async_function() {
        // Test async functions
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use cgminer_rs::*;

#[tokio::test]
async fn test_end_to_end_workflow() {
    // Test complete workflows
}
```

### Benchmarks

```rust
// benches/your_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("function_name", |b| {
        b.iter(|| {
            // Code to benchmark
        })
    });
}

criterion_group!(benches, benchmark_function);
criterion_main!(benches);
```

## Documentation Guidelines

### Code Documentation

```rust
/// Brief description of the function
/// 
/// # Arguments
/// 
/// * `param1` - Description of parameter 1
/// * `param2` - Description of parameter 2
/// 
/// # Returns
/// 
/// Description of return value
/// 
/// # Errors
/// 
/// Description of possible errors
/// 
/// # Examples
/// 
/// ```
/// use cgminer_rs::*;
/// 
/// let result = function_name(param1, param2)?;
/// assert_eq!(result, expected);
/// ```
pub fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, Error> {
    // Implementation
}
```

### API Documentation

Update `docs/API.md` with:
- Endpoint description
- Request/response examples
- Error codes
- Usage examples

## Performance Considerations

### Optimization Guidelines

1. **Profile Before Optimizing**
   ```bash
   make profile
   make flamegraph
   ```

2. **Use Appropriate Data Structures**
   - `Vec` for sequential access
   - `HashMap` for key-value lookups
   - `BTreeMap` for ordered data

3. **Minimize Allocations**
   - Reuse buffers where possible
   - Use `&str` instead of `String` when possible
   - Consider using `Cow<str>` for conditional ownership

4. **Async Best Practices**
   - Use `tokio::spawn` for CPU-intensive tasks
   - Prefer `async`/`await` over blocking operations
   - Use channels for communication between tasks

## Security Guidelines

### Input Validation

```rust
fn validate_input(input: &str) -> Result<(), ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::Empty);
    }
    
    if input.len() > MAX_LENGTH {
        return Err(ValidationError::TooLong);
    }
    
    // Additional validation
    Ok(())
}
```

### Error Handling

```rust
// Don't expose internal details
match internal_operation() {
    Ok(result) => Ok(result),
    Err(_) => Err(PublicError::OperationFailed),
}
```

## Release Process

### Version Numbering

Follow Semantic Versioning (SemVer):
- `MAJOR.MINOR.PATCH`
- Major: Breaking changes
- Minor: New features (backward compatible)
- Patch: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Update documentation
5. Create release PR
6. Tag release after merge
7. Build and publish artifacts

## Getting Help

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and discussions
- **Discord**: Real-time chat and community support

### Reporting Issues

When reporting bugs, include:
- CGMiner-RS version
- Operating system and version
- Hardware configuration
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs

### Feature Requests

When requesting features:
- Clear description of the feature
- Use case and motivation
- Proposed implementation (if any)
- Willingness to contribute

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to CGMiner-RS!
