#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#import <stdbool.h>
#import <stdint.h>
#import <sys/sysctl.h>

typedef bool (*ADGuideGranted)(uint8_t step);

static const uint8_t ADGuideAccessibility = 1;
static const uint8_t ADGuideScreenRecording = 2;
static const NSTimeInterval ADGuideLifetime = 600;
static const NSTimeInterval ADGuideDropRetry = 15;
static const CGFloat ADGuideWidth = 320;
static const CGFloat ADGuideGap = 16;
static const CGFloat ADGuideIconSize = 64;
static const CGFloat ADGuideGhostSize = 48;
static const int ADGuideMaxAncestors = 64;

static pid_t ADGuideParent(pid_t pid) {
    struct kinfo_proc info;
    size_t size = sizeof(info);
    int mib[4] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, pid};
    if (sysctl(mib, 4, &info, &size, NULL, 0) != 0 || size == 0) {
        return 0;
    }
    return info.kp_eproc.e_ppid;
}

bool agent_desktop_permission_guide_host(char *output, size_t capacity) {
    @autoreleasepool {
        pid_t pid = getpid();
        for (int depth = 0; depth < ADGuideMaxAncestors && pid > 1; depth++) {
            NSRunningApplication *app =
                [NSRunningApplication runningApplicationWithProcessIdentifier:pid];
            NSString *path = app.bundleURL.path;
            if ([path.pathExtension isEqualToString:@"app"]) {
                return [path getCString:output maxLength:capacity encoding:NSUTF8StringEncoding];
            }
            pid = ADGuideParent(pid);
        }
        return false;
    }
}

static NSRect ADGuideSettingsFrame(void) {
    NSArray<NSRunningApplication *> *apps =
        [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.systempreferences"];
    if (apps.count == 0) {
        return NSZeroRect;
    }
    pid_t pid = apps.firstObject.processIdentifier;
    CFArrayRef list = CGWindowListCopyWindowInfo(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID);
    if (list == NULL) {
        return NSZeroRect;
    }
    NSRect best = NSZeroRect;
    for (NSDictionary *window in (__bridge NSArray *)list) {
        if ([window[(id)kCGWindowOwnerPID] intValue] != pid ||
            [window[(id)kCGWindowLayer] intValue] != 0) {
            continue;
        }
        CGRect bounds;
        if (!CGRectMakeWithDictionaryRepresentation(
                (__bridge CFDictionaryRef)window[(id)kCGWindowBounds], &bounds)) {
            continue;
        }
        if (bounds.size.width * bounds.size.height > best.size.width * best.size.height) {
            best = bounds;
        }
    }
    CFRelease(list);
    if (NSIsEmptyRect(best)) {
        return NSZeroRect;
    }
    CGFloat primaryHeight = NSScreen.screens.firstObject.frame.size.height;
    best.origin.y = primaryHeight - best.origin.y - best.size.height;
    return best;
}

static NSScreen *ADGuideScreenFor(NSRect frame) {
    NSPoint center = NSMakePoint(NSMidX(frame), NSMidY(frame));
    for (NSScreen *screen in NSScreen.screens) {
        if (NSPointInRect(center, screen.frame)) {
            return screen;
        }
    }
    return NSScreen.mainScreen;
}

@interface ADGuideIcon : NSImageView <NSDraggingSource>
@property(nonatomic, copy) NSURL *fileURL;
@property(nonatomic, copy) void (^onDrop)(void);
@end

@implementation ADGuideIcon
- (BOOL)acceptsFirstMouse:(NSEvent *)__unused event {
    return YES;
}

- (void)resetCursorRects {
    [self addCursorRect:self.bounds cursor:NSCursor.openHandCursor];
}

- (NSDragOperation)draggingSession:(NSDraggingSession *)__unused session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)__unused context {
    return NSDragOperationCopy | NSDragOperationLink | NSDragOperationGeneric;
}

- (void)mouseDown:(NSEvent *)event {
    if (self.fileURL == nil) {
        return;
    }
    NSDraggingItem *item = [[NSDraggingItem alloc] initWithPasteboardWriter:self.fileURL];
    [item setDraggingFrame:self.bounds contents:self.image];
    [self beginDraggingSessionWithItems:@[ item ] event:event source:self];
}

- (void)draggingSession:(NSDraggingSession *)__unused session
           endedAtPoint:(NSPoint)__unused point
              operation:(NSDragOperation)operation {
    if (operation != NSDragOperationNone && self.onDrop != nil) {
        self.onDrop();
    }
}
@end

@interface ADGuidePanel : NSPanel
@end

@implementation ADGuidePanel
- (BOOL)canBecomeKeyWindow {
    return YES;
}
@end

@interface ADGuideButton : NSButton
@end

@implementation ADGuideButton
- (BOOL)acceptsFirstMouse:(NSEvent *)__unused event {
    return YES;
}

- (void)mouseDown:(NSEvent *)__unused event {
    [self sendAction:self.action to:self.target];
}
@end

@interface ADGuide : NSObject
@property(nonatomic, strong) NSPanel *panel;
@property(nonatomic, strong) NSVisualEffectView *card;
@property(nonatomic, strong) NSWindow *stage;
@property(nonatomic, strong) CALayer *carrier;
@property(nonatomic, strong) NSTextField *stepLabel;
@property(nonatomic, strong) NSTextField *titleLabel;
@property(nonatomic, strong) NSTextField *bodyLabel;
@property(nonatomic, strong) NSTextField *captionLabel;
@property(nonatomic, strong) ADGuideIcon *icon;
@property(nonatomic, strong) NSImage *hostImage;
@property(nonatomic, copy) NSString *hostName;
@property(nonatomic, strong) NSDate *deadline;
@property(nonatomic, strong) NSDate *droppedAt;
@property(nonatomic) BOOL coversScreenRecording;
@property(nonatomic) ADGuideGranted granted;
@property(nonatomic) uint8_t remaining;
@property(nonatomic) uint8_t current;
@property(nonatomic) int total;
@property(nonatomic) int completed;
@property(nonatomic) BOOL advancing;
@property(nonatomic) NSRect lastSettings;
@end

@implementation ADGuide
- (instancetype)initWithSteps:(uint8_t)steps host:(NSString *)hostPath granted:(ADGuideGranted)granted {
    self = [super init];
    if (self == nil) {
        return nil;
    }
    _granted = granted;
    _remaining = steps;
    _coversScreenRecording = (steps & ADGuideScreenRecording) != 0;
    _total = ((steps & ADGuideAccessibility) ? 1 : 0) + ((steps & ADGuideScreenRecording) ? 1 : 0);
    _deadline = [NSDate dateWithTimeIntervalSinceNow:ADGuideLifetime];
    _hostImage = [NSWorkspace.sharedWorkspace iconForFile:hostPath];
    _hostName = [[NSFileManager.defaultManager displayNameAtPath:hostPath] stringByDeletingPathExtension];
    [self buildPanel:[NSURL fileURLWithPath:hostPath]];
    [self buildStage];
    return self;
}

- (void)buildPanel:(NSURL *)hostURL {
    self.panel = [[ADGuidePanel alloc] initWithContentRect:NSMakeRect(0, 0, ADGuideWidth, 300)
                                            styleMask:NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel
                                              backing:NSBackingStoreBuffered
                                                defer:NO];
    self.panel.level = NSFloatingWindowLevel;
    self.panel.opaque = NO;
    self.panel.backgroundColor = NSColor.clearColor;
    self.panel.hasShadow = YES;
    self.panel.movableByWindowBackground = YES;
    self.panel.hidesOnDeactivate = NO;
    self.panel.collectionBehavior =
        NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary;
    self.card = [[NSVisualEffectView alloc] init];
    self.card.material = NSVisualEffectMaterialHUDWindow;
    self.card.state = NSVisualEffectStateActive;
    self.card.wantsLayer = YES;
    self.card.layer.cornerRadius = 14;
    self.card.layer.masksToBounds = YES;
    self.panel.contentView = self.card;
    self.stepLabel = [NSTextField labelWithString:@""];
    self.stepLabel.font = [NSFont systemFontOfSize:11 weight:NSFontWeightMedium];
    self.stepLabel.textColor = NSColor.secondaryLabelColor;
    self.titleLabel = [NSTextField labelWithString:@""];
    self.titleLabel.font = [NSFont systemFontOfSize:16 weight:NSFontWeightSemibold];
    self.icon = [[ADGuideIcon alloc] init];
    self.icon.fileURL = hostURL;
    self.icon.image = self.hostImage;
    self.icon.imageScaling = NSImageScaleProportionallyUpOrDown;
    [self.icon.widthAnchor constraintEqualToConstant:ADGuideIconSize].active = YES;
    [self.icon.heightAnchor constraintEqualToConstant:ADGuideIconSize].active = YES;
    __weak ADGuide *weakSelf = self;
    self.icon.onDrop = ^{
        [weakSelf showDropped];
    };
    self.captionLabel = [NSTextField labelWithString:[NSString stringWithFormat:@"Drag %@", self.hostName]];
    self.captionLabel.font = [NSFont systemFontOfSize:11];
    self.captionLabel.textColor = NSColor.secondaryLabelColor;
    self.bodyLabel = [NSTextField wrappingLabelWithString:@""];
    self.bodyLabel.alignment = NSTextAlignmentCenter;
    self.bodyLabel.preferredMaxLayoutWidth = ADGuideWidth - 40;
    self.bodyLabel.textColor = NSColor.secondaryLabelColor;
    NSButton *close = [ADGuideButton buttonWithTitle:@"Not Now" target:self action:@selector(finish)];
    close.bezelStyle = NSBezelStyleRounded;
    NSStackView *stack = [NSStackView stackViewWithViews:@[
        self.stepLabel, self.titleLabel, self.icon, self.captionLabel, self.bodyLabel, close
    ]];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeCenterX;
    stack.spacing = 8;
    stack.edgeInsets = NSEdgeInsetsMake(18, 20, 18, 20);
    [stack setCustomSpacing:14 afterView:self.titleLabel];
    [stack setCustomSpacing:14 afterView:self.bodyLabel];
    stack.translatesAutoresizingMaskIntoConstraints = NO;
    [self.card addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.leadingAnchor constraintEqualToAnchor:self.card.leadingAnchor],
        [stack.trailingAnchor constraintEqualToAnchor:self.card.trailingAnchor],
        [stack.topAnchor constraintEqualToAnchor:self.card.topAnchor],
        [stack.bottomAnchor constraintEqualToAnchor:self.card.bottomAnchor],
        [stack.widthAnchor constraintEqualToConstant:ADGuideWidth],
    ]];
}

- (void)buildStage {
    self.stage = [[NSWindow alloc] initWithContentRect:NSZeroRect
                                             styleMask:NSWindowStyleMaskBorderless
                                               backing:NSBackingStoreBuffered
                                                 defer:NO];
    self.stage.opaque = NO;
    self.stage.backgroundColor = NSColor.clearColor;
    self.stage.ignoresMouseEvents = YES;
    self.stage.hasShadow = NO;
    self.stage.level = NSStatusWindowLevel;
    self.stage.collectionBehavior = self.panel.collectionBehavior;
    self.stage.contentView.wantsLayer = YES;
    self.carrier = [CALayer layer];
    self.carrier.bounds = CGRectMake(0, 0, ADGuideGhostSize, ADGuideGhostSize);
    self.carrier.opacity = 0;
    CALayer *ghost = [CALayer layer];
    ghost.frame = self.carrier.bounds;
    ghost.contents = self.hostImage;
    ghost.opacity = 0.85;
    CALayer *hand = [CALayer layer];
    NSImage *handImage = NSCursor.closedHandCursor.image;
    hand.contents = handImage;
    hand.frame = CGRectMake(ADGuideGhostSize / 2, ADGuideGhostSize / 2 - handImage.size.height,
                            handImage.size.width, handImage.size.height);
    [self.carrier addSublayer:ghost];
    [self.carrier addSublayer:hand];
    [self.stage.contentView.layer addSublayer:self.carrier];
}

- (void)start {
    [self beginStep];
    [self.panel center];
    [self follow:ADGuideSettingsFrame()];
    [self.panel orderFrontRegardless];
    [NSTimer scheduledTimerWithTimeInterval:1
                                    repeats:YES
                                      block:^(NSTimer *__unused timer) {
                                        [self tick];
                                      }];
}

- (void)beginStep {
    self.current = (self.remaining & ADGuideAccessibility) ? ADGuideAccessibility : ADGuideScreenRecording;
    BOOL accessibility = self.current == ADGuideAccessibility;
    self.stepLabel.stringValue =
        [NSString stringWithFormat:@"Step %d of %d", self.completed + 1, self.total];
    self.titleLabel.stringValue = accessibility ? @"Allow Accessibility" : @"Allow Screen Recording";
    [self showInstructions];
    NSString *pane = accessibility ? @"Privacy_Accessibility" : @"Privacy_ScreenCapture";
    NSOperatingSystemVersion ventura = {13, 0, 0};
    NSString *anchor = [NSProcessInfo.processInfo isOperatingSystemAtLeastVersion:ventura]
        ? @"x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?"
        : @"x-apple.systempreferences:com.apple.preference.security?";
    NSString *link = [anchor stringByAppendingString:pane];
    [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:link]];
    self.advancing = NO;
}

- (void)showInstructions {
    self.bodyLabel.stringValue = [NSString
        stringWithFormat:@"Drag %@ into the list, then turn on its switch. If %@ is already in the list, just turn on its switch.",
                         self.hostName, self.hostName];
    self.icon.image = self.hostImage;
    self.icon.contentTintColor = nil;
    self.captionLabel.hidden = NO;
    self.droppedAt = nil;
    self.lastSettings = NSZeroRect;
    [self resizePanel];
}

- (void)retryStalledDrop {
    if (self.droppedAt != nil && [self.droppedAt timeIntervalSinceNow] < -ADGuideDropRetry) {
        [self showInstructions];
    }
}

- (void)resizePanel {
    NSRect frame = self.panel.frame;
    NSSize size = self.card.fittingSize;
    frame.origin.y += frame.size.height - size.height;
    frame.size = size;
    [self.panel setFrame:frame display:YES];
}

- (void)tick {
    if ([self.deadline timeIntervalSinceNow] <= 0) {
        [self finish];
        return;
    }
    [self follow:ADGuideSettingsFrame()];
    if (self.advancing) {
        return;
    }
    if (!self.granted(self.current)) {
        [self retryStalledDrop];
        return;
    }
    self.advancing = YES;
    [self showGranted];
    self.remaining &= (uint8_t)~self.current;
    self.completed += 1;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(1.2 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        if (self.remaining == 0) {
            [self showDone];
        } else {
            [self beginStep];
        }
    });
}

- (void)follow:(NSRect)settings {
    if (NSEqualRects(settings, self.lastSettings)) {
        return;
    }
    self.lastSettings = settings;
    if (NSIsEmptyRect(settings)) {
        [self.stage orderOut:nil];
        return;
    }
    NSRect area = ADGuideScreenFor(settings).visibleFrame;
    NSRect frame = self.panel.frame;
    frame.origin.x = NSMaxX(settings) + ADGuideGap;
    if (NSMaxX(frame) > NSMaxX(area)) {
        frame.origin.x = NSMinX(settings) - ADGuideGap - frame.size.width;
    }
    frame.origin.x = MAX(NSMinX(area), MIN(frame.origin.x, NSMaxX(area) - frame.size.width));
    frame.origin.y = MAX(NSMinY(area), NSMaxY(settings) - frame.size.height);
    [self.panel setFrame:frame display:YES];
    [self animateInto:settings];
}

- (void)animateInto:(NSRect)settings {
    if (self.captionLabel.isHidden || NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion) {
        [self.stage orderOut:nil];
        return;
    }
    NSRect screen = ADGuideScreenFor(settings).frame;
    [self.stage setFrame:screen display:NO];
    NSRect iconRect = [self.panel convertRectToScreen:[self.icon convertRect:self.icon.bounds toView:nil]];
    NSPoint from = NSMakePoint(NSMidX(iconRect) - screen.origin.x, NSMidY(iconRect) - screen.origin.y);
    NSPoint to = NSMakePoint(NSMinX(settings) + settings.size.width * 0.62 - screen.origin.x,
                             NSMinY(settings) + settings.size.height * 0.55 - screen.origin.y);
    CAKeyframeAnimation *move = [CAKeyframeAnimation animationWithKeyPath:@"position"];
    move.values = @[
        [NSValue valueWithPoint:from], [NSValue valueWithPoint:from], [NSValue valueWithPoint:to],
        [NSValue valueWithPoint:to]
    ];
    move.keyTimes = @[ @0, @0.15, @0.7, @1 ];
    move.timingFunctions = @[
        [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionLinear],
        [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseInEaseOut],
        [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionLinear],
    ];
    CAKeyframeAnimation *fade = [CAKeyframeAnimation animationWithKeyPath:@"opacity"];
    fade.values = @[ @0, @1, @1, @0 ];
    fade.keyTimes = @[ @0, @0.1, @0.8, @1 ];
    CAKeyframeAnimation *lift = [CAKeyframeAnimation animationWithKeyPath:@"transform.scale"];
    lift.values = @[ @1, @1.12, @1.12, @0.9 ];
    lift.keyTimes = @[ @0, @0.15, @0.7, @1 ];
    CAAnimationGroup *group = [CAAnimationGroup animation];
    group.animations = @[ move, fade, lift ];
    group.duration = 2.8;
    group.repeatCount = HUGE_VALF;
    [self.carrier removeAllAnimations];
    [self.carrier addAnimation:group forKey:@"guide"];
    [self.stage orderFrontRegardless];
}

- (void)showDropped {
    self.droppedAt = [NSDate date];
    self.captionLabel.hidden = YES;
    [self.stage orderOut:nil];
    self.bodyLabel.stringValue =
        [NSString stringWithFormat:@"Now turn on the switch next to %@.", self.hostName];
    [self resizePanel];
}

- (void)showGranted {
    [self.stage orderOut:nil];
    self.captionLabel.hidden = YES;
    if (@available(macOS 11.0, *)) {
        NSImage *check = [NSImage imageWithSystemSymbolName:@"checkmark.circle.fill"
                                   accessibilityDescription:@"Granted"];
        NSImageSymbolConfiguration *config =
            [NSImageSymbolConfiguration configurationWithPointSize:48 weight:NSFontWeightRegular];
        self.icon.image = [check imageWithSymbolConfiguration:config];
        self.icon.contentTintColor = NSColor.systemGreenColor;
    }
    self.bodyLabel.stringValue = @"Done.";
    [self resizePanel];
}

- (void)showDone {
    self.stepLabel.stringValue = @"";
    self.titleLabel.stringValue = @"You are all set";
    self.bodyLabel.stringValue = self.coversScreenRecording
        ? [NSString stringWithFormat:@"Quit and reopen %@ before agent-desktop takes screenshots.", self.hostName]
        : @"agent-desktop can now see and use your apps.";
    [self resizePanel];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(2 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        [self finish];
    });
}

- (void)finish {
    [NSApp terminate:nil];
}
@end

int agent_desktop_permission_guide_run(uint8_t steps, const char *host_path, ADGuideGranted granted) {
    @autoreleasepool {
        NSString *host = host_path == NULL ? nil : [NSString stringWithUTF8String:host_path];
        if (steps == 0 || host.length == 0 || granted == NULL) {
            return 1;
        }
        NSApplication *app = NSApplication.sharedApplication;
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];
        ADGuide *guide = [[ADGuide alloc] initWithSteps:steps host:host granted:granted];
        [guide start];
        [app run];
        return 0;
    }
}
