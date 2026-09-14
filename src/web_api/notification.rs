// Web Notification API implementation for Web standard
// v0.3.328: Notification API for displaying system notifications
// Provides Notification constructor, permission management, and instance methods

use anyhow::Result;
use rusty_v8 as v8;

/// Notification permission state
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationPermission {
    Granted, // User has granted permission
    Denied,  // User has denied permission
    Default, // User has not been asked yet
}

impl NotificationPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationPermission::Granted => "granted",
            NotificationPermission::Denied => "denied",
            NotificationPermission::Default => "default",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "granted" => NotificationPermission::Granted,
            "denied" => NotificationPermission::Denied,
            _ => NotificationPermission::Default,
        }
    }
}

/// Setup Notification API in V8 context
pub fn setup_notification_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
    _global: v8::Local<v8::Object>,
) -> Result<()> {
    let global = context.global(scope);

    // Notification constructor
    let notification_fn = v8::FunctionTemplate::new(scope, notification_constructor_callback);
    let notification_constructor = notification_fn.get_function(scope).unwrap();
    let notification_key = v8::String::new(scope, "Notification").unwrap();
    global.set(
        scope,
        notification_key.into(),
        notification_constructor.into(),
    );

    // Notification.permission (static property)
    let permission_key = v8::String::new(scope, "permission").unwrap();
    let permission_val = v8::String::new(scope, "default").unwrap();
    notification_constructor.set(scope, permission_key.into(), permission_val.into());

    // Notification.requestPermission (static method)
    let request_perm_fn =
        v8::FunctionTemplate::new(scope, notification_request_permission_callback);
    let request_perm_key = v8::String::new(scope, "requestPermission").unwrap();
    let request_perm_func = request_perm_fn.get_function(scope).unwrap();
    notification_constructor.set(scope, request_perm_key.into(), request_perm_func.into());

    Ok(())
}

/// Notification constructor callback
fn notification_constructor_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let message = v8::String::new(scope, "Notification permission is not granted").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
    rv.set(v8::undefined(scope).into());
}

/// Notification.requestPermission() callback
fn notification_request_permission_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Create a promise that resolves to the permission
    let resolver = match v8::PromiseResolver::new(scope) {
        Some(r) => r,
        None => {
            let error = v8::String::new(scope, "Failed to create promise resolver").unwrap();
            scope.throw_exception(error.into());
            return;
        }
    };
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    let permission = "default";
    let permission_str = v8::String::new(scope, permission).unwrap();
    resolver.resolve(scope, permission_str.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_permission_values() {
        assert_eq!(NotificationPermission::Granted.as_str(), "granted");
        assert_eq!(NotificationPermission::Denied.as_str(), "denied");
        assert_eq!(NotificationPermission::Default.as_str(), "default");
    }

    #[test]
    fn test_notification_permission_from_string() {
        assert_eq!(
            NotificationPermission::from_string("granted"),
            NotificationPermission::Granted
        );
        assert_eq!(
            NotificationPermission::from_string("denied"),
            NotificationPermission::Denied
        );
        assert_eq!(
            NotificationPermission::from_string("unknown"),
            NotificationPermission::Default
        );
        assert_eq!(
            NotificationPermission::from_string("GRANTED"),
            NotificationPermission::Granted
        );
    }

    #[test]
    fn test_notification_permission_default_case() {
        // Test that case-insensitive matching works
        assert_eq!(
            NotificationPermission::from_string("Default"),
            NotificationPermission::Default
        );
        assert_eq!(
            NotificationPermission::from_string("DEFAULT"),
            NotificationPermission::Default
        );
    }
}
