# EBD feature and plugin plan

Status key: `[ ]` planned, `[x]` done, `[?]` needs review before building.

Derived from the community plugin list (generated 2026-11-17). Descriptions were OCR-extracted and partially reconstructed; verify intent before implementing any plugin.

## Project rules

- Core depends only on the stable `ClientAdapter` contract, never Discord internals.
- Every adapter must declare capabilities; plugins get explicit permissions and run sandboxed (WASM).
- No undocumented injection, credential extraction, or bypass of client security.
- Discord integration stays disabled until an authorised, documented interface exists.

Suggested per-plugin fields: capability needed, permission, adapter hook, priority.



## UI & APPEARANCE


- [ ] **AlwaysAnimate**
  Animates anything that can be animated.

- [ ] **BetterFolders**
  Shows server folders on a dedicated sidebar and adds folder-related improvements.

- [ ] **PlainFolderIcon**
  Hides the small guild icons shown inside folders.

- [ ] **BetterGifAltText**
  Changes GIF alt text from 'GIF' to the GIF's tags/filename.

- [ ] **BetterRoleDot**
  Copies the role colour when the RoleDot (accessibility setting) is clicked; allows RoleDot and coloured names together.

- [ ] **BiggerStreamPreview**
  Lets you enlarge stream previews.

- [ ] **BlurNSFW**
  Blurs attachments in NSFW channels until hovered.

- [ ] **ClientTheme**
  Recreates the old client-theme experiment: add a colour to the client theme.

- [ ] **ColorSighted**
  Removes the colourblind-friendly icons from statuses (2015-2017 look).

- [ ] **FixCodeblockGap**
  Removes the gap between code blocks and the text below them.

- [ ] **FixImagesQuality**
  Loads images at original resolution for better quality.

- [ ] **IrcColors**
  Gives each username a unique chat colour, like IRC clients.

- [ ] **MentionAvatars**
  Shows user avatars and role icons inside mentions.

- [ ] **NoMosaic**
  Removes the image mosaic layout.

- [ ] **NoTypingAnimation**
  Disables the CPU-intensive typing dots animation.

- [ ] **PlatformIndicators**
  Shows platform indicators (Desktop, Mobile, Web) on users.

- [ ] **RoleColorEverywhere**
  Applies the top role colour everywhere possible.

- [ ] **ShikiCodeblocks**
  VS Code-style syntax-highlighted code blocks (powered by Shiki).

- [ ] **ThemeAttributes**
  Adds data attributes to elements for theming.

- [ ] **TypingTweaks**
  Shows avatars and role colours in the typing indicator.

- [ ] **Unindent**
  Trims leading indentation from code blocks.

- [ ] **ShowMeYourName**
  Displays usernames next to nicknames, or hides nicknames.

- [ ] **UserMessagesPronouns**
  Adds pronouns next to chat user messages.

- [ ] **ServerListIndicators**
  Adds online-friend count or server count in the server list.

- [ ] **MutedServersActivityBadge**
  Keeps the server activity badge visible even when the server is muted.

- [ ] **VencordToolbox**
  Adds a titlebar button holding quick actions (rename for EBD toolbox).


## SETTINGS & ORGANISATION


- [ ] **BetterSettings**
  Improves opening the settings menu. Options: Disable Fade (no crossfade), Organize Menu (categorised settings cog menu), Eager Load (no first-open delay).

- [ ] **Settings**
  Adds the plugin Settings UI and debug info. (Required)

- [ ] **StartupTimings**
  Adds startup timings to the Settings menu.

- [ ] **BetterSessions**
  Enhances the sessions/devices menu: exact timestamps, custom session names.

- [ ] **NewGuildSettings**
  Auto-mutes new servers and changes other settings on joining.

- [ ] **OverrideForumDefaults**
  Overrides default forum layout/sort order (per-channel changes still possible).

- [ ] **CustomIdle**
  Sets the time before going idle, or disables auto-idle.

- [ ] **KeepCurrentChannel**
  Returns you to the previous channel after switching accounts or reloading.

- [ ] **LoadingQuotes**
  Replaces the loading quotes.


## CHAT & MESSAGING


- [ ] **CharacterCounter**
  Adds a character counter to the chat input.

- [ ] **ClearURLs**
  Removes tracking elements from URLs you send.

- [ ] **CopyEmojiMarkdown**
  Copies emojis as formatted strings.

- [ ] **CopyFileContents**
  Adds a button on text-file attachments to copy contents.

- [ ] **CopyUserURLs**
  Adds 'Copy User URL' to the user context menu.

- [ ] **CustomCommands**
  Create custom slash commands / tags.

- [ ] **DontRoundMyTimestamps**
  Rounds relative timestamps down (7.6y becomes 7y).

- [ ] **FavoriteEmojiFirst**
  Puts favourite emoji first in autocomplete.

- [ ] **GifPaste**
  Picking a GIF inserts its link instead of sending.

- [ ] **StickerPaste**
  Picking a sticker inserts it into the chatbox instead of sending.

- [ ] **GifProviderSwitcher**
  Use Tenor or Giphy instead of the default GIF search.

- [ ] **BetterGifPicker**
  GIF picker opens on the Favourites category.

- [ ] **GreetStickerPicker**
  Choose any greet sticker instead of the random one.

- [ ] **HideMedia**
  Hide attachments/embeds on individual messages via hover button.

- [ ] **ImageFilename**
  Shows image/GIF file name as a tooltip on hover.

- [ ] **ImageLink**
  Never hides image links, even if they are the only content.

- [ ] **ImageZoom**
  Zoom images/GIFs: scroll to zoom, Shift+scroll to change lens size.

- [ ] **JumpToTop**
  Button to jump to the first message in a channel.

- [ ] **MessageClickActions**
  Hold Backspace + click to delete; double-click to edit/reply.

- [ ] **MessageLatency**
  Indicator for messages that took n+ seconds to send.

- [ ] **MessageLinkEmbeds**
  Preview for messages that link to another message.

- [ ] **MessageLogger**
  Temporarily logs deleted and edited messages.

- [ ] **MoreQuickReactions**
  Increases the number of Quick React options.

- [ ] **NoMaskedUrlPaste**
  Pasting a link over selected text does not create a masked URL.

- [ ] **NoReplyMention**
  Disables reply pings by default.

- [ ] **NoServerEmojis**
  Hides server emojis from autocomplete.

- [ ] **PreviewMessage**
  Preview your message before sending.

- [ ] **QuickMention**
  Quick mention button in the message actions bar.

- [ ] **QuickReply**
  Reply with Ctrl+Up/Down; edit with Ctrl+Shift+Up/Down.

- [ ] **ReplyTimestamp**
  Shows a timestamp on replied-message previews.

- [ ] **RevealAllSpoilers**
  Ctrl-click a spoiler to reveal all in a message; Ctrl+Shift-click in the chat.

- [ ] **SendTimestamps**
  Send timestamps via chat-box button and text shortcuts.

- [ ] **ShowAllMessageButtons**
  Always show all message buttons without holding Shift.

- [ ] **SilentMessageToggle**
  Chat-bar button to toggle silent messages.

- [ ] **SilentTyping**
  Hides that you are typing.

- [ ] **SuperReactionTweaks**
  Limit simultaneous Super Reactions; super-react by default.

- [ ] **TextReplace**
  Replace text in your messages using rules.

- [ ] **Translate**
  Translate messages via Google Translate, DeepL or Kagi.

- [ ] **TypingIndicator**
  Shows if someone is typing in a channel.

- [ ] **UnsuppressEmbeds**
  Allows unsuppressing embeds in messages.

- [ ] **ValidReply**
  Fixes 'Message could not be loaded' when hovering a reply.

- [ ] **ValidUser**
  Fixes '@unknown-user' mentions (hover to fix).

- [ ] **WhoReacted**
  Shows avatars of users who reacted.

- [ ] **petpet**
  Adds /petpet slash command to make head-pat GIFs from an image.

- [ ] **BetterSearch**
  Extra channel message search options.

- [ ] **NoBlockedMessages**
  Hides blocked/ignored messages completely.

- [ ] **NoUnblockToJump**
  Jump to messages from blocked/ignored users or likely spammers without unblocking.

- [ ] **iLoveSpam**
  Do not hide messages from 'likely spammers'.

- [ ] **OnePingPerDM**
  Only one audio ping for repeated unread DMs from the same user.

- [ ] **PinDMs**
  Pin private channels to the top of the DM list; right-click to pin/reorder.

- [ ] **ReviewDB**
  Review other users via a new profile section.

- [ ] **ViewRaw**
  Copy/view raw data of any message, channel or guild.


## MEDIA, FILES & EMBEDS


- [ ] **AnonymiseFileNames**
  Anonymises uploaded file names.

- [ ] **BetterUploadButton**
  Single click uploads; right click opens menu.

- [ ] **DirectFileDownload**
  Downloads files directly instead of opening in browser.

- [ ] **Dearrow**
  Makes YouTube embed titles/thumbnails less sensationalist (DeArrow).

- [ ] **FixSpotifyEmbeds**
  Lets you customise Spotify embed volume.

- [ ] **FixYoutubeEmbeds**
  Bypasses blocked YouTube videos in embeds.

- [ ] **PictureInPicture**
  Picture-in-picture for videos.

- [ ] **ReverseImageSearch**
  Adds image search to image context menus.

- [ ] **ReplaceGoogleSearch**
  Replaces Google search with other engine(s).

- [ ] **VoiceDownload**
  Adds download for voice messages.

- [ ] **VoiceMessages**
  Send voice messages like on mobile.

- [ ] **YoutubeAdblock**
  Blocks ads in YouTube embeds and Watch Together via AdGuard.

- [ ] **OpenInApp**
  Opens links in their native apps instead of the browser.

- [ ] **ViewIcons**
  Clickable avatars/banners; View Icon/Banner/Avatar Decoration entries.

- [ ] **UnlockedAvatarZoom**
  More zoom in the avatar crop tool.

- [ ] **USRBG**
  Shows USRBG user banners.


## VOICE & CALLS


- [ ] **CallTimer**
  Adds a timer to voice calls.

- [ ] **DisableCallIdle**
  Stops being kicked from a DM call after 3 minutes alone.

- [ ] **NotificationVolume**
  Separate volume for notifications and in-app sounds.

- [ ] **UserVoiceShow**
  Indicator when a user is in a voice channel.

- [ ] **VcNarrator**
  Narrates when users join, leave, or move voice channels.

- [ ] **VoiceChatDoubleClick**
  Join voice via double-click.

- [ ] **VolumeBooster**
  Set user/stream volume above the default maximum.

- [ ] **SecretRingToneEnabler**
  Always plays the secret ringtone version.

- [ ] **XSOverlay**
  Forwards notifications to XSOverlay for VR.

- [ ] **StreamerModeOnStream**
  Auto-enables streamer mode when streaming.


## SOCIAL, PROFILES & PRESENCE


- [ ] **AutoDNDWhilePlaying**
  Updates status (online/idle/dnd) when launching games.

- [ ] **CustomRPC**
  Fully customisable Rich Presence.

- [ ] **GameActivityToggle**
  Button near mic/deafen to toggle game activity (right-click for Spotify).

- [ ] **IgnoreActivities**
  Hide chosen activities from your status only.

- [ ] **MusicRichPresence**
  Rich Presence for Last.fm / ListenBrainz.

- [ ] **ImplicitRelationships**
  Shows implicit relationships in the Friends tab.

- [ ] **MutualGroupDMs**
  Shows mutual group DMs in profiles.

- [ ] **NoPendingCount**
  Removes ping counts for friend/message requests and offers.

- [ ] **NoSystemBadge**
  Disables taskbar/tray unread badge.

- [ ] **ReadAllNotificationsButton**
  One click to read all server notifications.

- [ ] **RelationshipNotifier**
  Notifies when a friend, group chat or server removes you.

- [ ] **ShowConnections**
  Shows connected accounts in user popouts and DM sidebar.

- [ ] **SortFriendRequests**
  Sorts friend requests by date received.

- [ ] **SpotifyControls**
  Spotify player above the account panel.

- [ ] **SpotifyShareCommands**
  Share current track/album/artist via /track, /album, /artist.

- [ ] **AlwaysExpandRoles**
  Always expands the role list in profile popouts.

- [ ] **BetterRoleContext**
  Copy role colour / edit role / view role icon via role right-click.

- [ ] **ForceOwnerCrown**
  Shows owner crown even in large servers.

- [ ] **MemberCount**
  Shows online, total members and voice users.

- [ ] **ServerInfo**
  View info about a server.

- [ ] **ShowTimeoutDuration**
  Shows remaining time on a user's timeout.

- [ ] **PermissionsViewer**
  View permissions of a user/channel and server roles.

- [ ] **ExpressionManager**
  Clone emotes/stickers; quick clone, edit, delete.

- [ ] **NoOnboardingDelay**
  Skips the onboarding delay.

- [ ] **PauseInvitesForever**
  Restores option to pause invites indefinitely.

- [ ] **AlwaysTrust**
  Removes untrusted-domain and suspicious-file popups.

- [ ] **oneko**
  Cat that follows the mouse (fun).


## DEVELOPER & UTILITY


- [ ] **ConcatenatedComponentExt**
  Extracts components concatenated by the bundler. (Required)

- [ ] **FixDiscordCss**
  Fixes CSS lag. (Required)

- [ ] **NoTrack**
  Disables analytics/'science', metrics and Sentry crash reporting. (Required)

- [ ] **SupportHelper**
  Helps provide support to users. (Required)

- [ ] **CrashHandler**
  Handles and may recover from crashes without restart.

- [ ] **ConsoleJanitor**
  Silences annoying console messages/errors.

- [ ] **ConsoleShortcuts**
  Short aliases on window; run 'shortcutList'.

- [ ] **F8Break**
  Pause the client on F8 with DevTools open.

- [ ] **NoDevtoolsWarning**
  Disables the console 'HOLD UP' banner.

- [ ] **NoF1**
  Disables the F1 help bind.

- [ ] **ReactErrorDecoder**
  Replaces 'Minified React Error' with the real error.


## RISKY / REVIEW BEFORE BUILDING


- [?] **FakeNitro**
  Send fake emojis/stickers, Nitro themes, Nitro-quality streaming. Likely conflicts with paid-feature/ToS rules.

- [?] **FakeProfileThemes**
  Profile themes hidden in bio via invisible encoding.

- [?] **SpotifyCrack**
  Free listen-along and no auto-pause. Bypasses Spotify restrictions.

- [?] **Experiments**
  Enables experiments and dev-only features.

- [?] **PermissionFreeWill**
  Disables client-side permission-management restrictions.

- [?] **ShowHiddenChannels**
  Shows channels you cannot access.

- [?] **ShowHiddenThings**
  Shows hidden/moderator-only items regardless of permissions.

- [?] **NoProfileThemes**
  Removes Nitro profile themes from everyone but you.



**TOTAL UNIQUE PLUGINS: 159**

> NOTE: Source text was OCR-extracted; some descriptions were truncated and completed from the plugin names. Verify before implementing.
