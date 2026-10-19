// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'core.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CoreEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent()';
}


}

/// @nodoc
class $CoreEventCopyWith<$Res>  {
$CoreEventCopyWith(CoreEvent _, $Res Function(CoreEvent) __);
}


/// Adds pattern-matching-related methods to [CoreEvent].
extension CoreEventPatterns on CoreEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CoreEvent_DevicesChanged value)?  devicesChanged,TResult Function( CoreEvent_Paired value)?  paired,TResult Function( CoreEvent_ReceiveStarted value)?  receiveStarted,TResult Function( CoreEvent_ReceiveProgress value)?  receiveProgress,TResult Function( CoreEvent_Saved value)?  saved,TResult Function( CoreEvent_FlingUrl value)?  flingUrl,TResult Function( CoreEvent_FlingText value)?  flingText,TResult Function( CoreEvent_ReceiveFinished value)?  receiveFinished,TResult Function( CoreEvent_ReceiveInterrupted value)?  receiveInterrupted,TResult Function( CoreEvent_Linked value)?  linked,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged() when devicesChanged != null:
return devicesChanged(_that);case CoreEvent_Paired() when paired != null:
return paired(_that);case CoreEvent_ReceiveStarted() when receiveStarted != null:
return receiveStarted(_that);case CoreEvent_ReceiveProgress() when receiveProgress != null:
return receiveProgress(_that);case CoreEvent_Saved() when saved != null:
return saved(_that);case CoreEvent_FlingUrl() when flingUrl != null:
return flingUrl(_that);case CoreEvent_FlingText() when flingText != null:
return flingText(_that);case CoreEvent_ReceiveFinished() when receiveFinished != null:
return receiveFinished(_that);case CoreEvent_ReceiveInterrupted() when receiveInterrupted != null:
return receiveInterrupted(_that);case CoreEvent_Linked() when linked != null:
return linked(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CoreEvent_DevicesChanged value)  devicesChanged,required TResult Function( CoreEvent_Paired value)  paired,required TResult Function( CoreEvent_ReceiveStarted value)  receiveStarted,required TResult Function( CoreEvent_ReceiveProgress value)  receiveProgress,required TResult Function( CoreEvent_Saved value)  saved,required TResult Function( CoreEvent_FlingUrl value)  flingUrl,required TResult Function( CoreEvent_FlingText value)  flingText,required TResult Function( CoreEvent_ReceiveFinished value)  receiveFinished,required TResult Function( CoreEvent_ReceiveInterrupted value)  receiveInterrupted,required TResult Function( CoreEvent_Linked value)  linked,}){
final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged():
return devicesChanged(_that);case CoreEvent_Paired():
return paired(_that);case CoreEvent_ReceiveStarted():
return receiveStarted(_that);case CoreEvent_ReceiveProgress():
return receiveProgress(_that);case CoreEvent_Saved():
return saved(_that);case CoreEvent_FlingUrl():
return flingUrl(_that);case CoreEvent_FlingText():
return flingText(_that);case CoreEvent_ReceiveFinished():
return receiveFinished(_that);case CoreEvent_ReceiveInterrupted():
return receiveInterrupted(_that);case CoreEvent_Linked():
return linked(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CoreEvent_DevicesChanged value)?  devicesChanged,TResult? Function( CoreEvent_Paired value)?  paired,TResult? Function( CoreEvent_ReceiveStarted value)?  receiveStarted,TResult? Function( CoreEvent_ReceiveProgress value)?  receiveProgress,TResult? Function( CoreEvent_Saved value)?  saved,TResult? Function( CoreEvent_FlingUrl value)?  flingUrl,TResult? Function( CoreEvent_FlingText value)?  flingText,TResult? Function( CoreEvent_ReceiveFinished value)?  receiveFinished,TResult? Function( CoreEvent_ReceiveInterrupted value)?  receiveInterrupted,TResult? Function( CoreEvent_Linked value)?  linked,}){
final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged() when devicesChanged != null:
return devicesChanged(_that);case CoreEvent_Paired() when paired != null:
return paired(_that);case CoreEvent_ReceiveStarted() when receiveStarted != null:
return receiveStarted(_that);case CoreEvent_ReceiveProgress() when receiveProgress != null:
return receiveProgress(_that);case CoreEvent_Saved() when saved != null:
return saved(_that);case CoreEvent_FlingUrl() when flingUrl != null:
return flingUrl(_that);case CoreEvent_FlingText() when flingText != null:
return flingText(_that);case CoreEvent_ReceiveFinished() when receiveFinished != null:
return receiveFinished(_that);case CoreEvent_ReceiveInterrupted() when receiveInterrupted != null:
return receiveInterrupted(_that);case CoreEvent_Linked() when linked != null:
return linked(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  devicesChanged,TResult Function( String name)?  paired,TResult Function( String from,  int files,  BigInt totalBytes,  BigInt alreadyHave)?  receiveStarted,TResult Function( BigInt received,  BigInt total)?  receiveProgress,TResult Function( String path,  bool open,  int? page)?  saved,TResult Function( String from,  String url,  String title)?  flingUrl,TResult Function( String from,  String text)?  flingText,TResult Function()?  receiveFinished,TResult Function( String reason)?  receiveInterrupted,TResult Function( String id,  LinkedEventKind kind)?  linked,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged() when devicesChanged != null:
return devicesChanged();case CoreEvent_Paired() when paired != null:
return paired(_that.name);case CoreEvent_ReceiveStarted() when receiveStarted != null:
return receiveStarted(_that.from,_that.files,_that.totalBytes,_that.alreadyHave);case CoreEvent_ReceiveProgress() when receiveProgress != null:
return receiveProgress(_that.received,_that.total);case CoreEvent_Saved() when saved != null:
return saved(_that.path,_that.open,_that.page);case CoreEvent_FlingUrl() when flingUrl != null:
return flingUrl(_that.from,_that.url,_that.title);case CoreEvent_FlingText() when flingText != null:
return flingText(_that.from,_that.text);case CoreEvent_ReceiveFinished() when receiveFinished != null:
return receiveFinished();case CoreEvent_ReceiveInterrupted() when receiveInterrupted != null:
return receiveInterrupted(_that.reason);case CoreEvent_Linked() when linked != null:
return linked(_that.id,_that.kind);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  devicesChanged,required TResult Function( String name)  paired,required TResult Function( String from,  int files,  BigInt totalBytes,  BigInt alreadyHave)  receiveStarted,required TResult Function( BigInt received,  BigInt total)  receiveProgress,required TResult Function( String path,  bool open,  int? page)  saved,required TResult Function( String from,  String url,  String title)  flingUrl,required TResult Function( String from,  String text)  flingText,required TResult Function()  receiveFinished,required TResult Function( String reason)  receiveInterrupted,required TResult Function( String id,  LinkedEventKind kind)  linked,}) {final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged():
return devicesChanged();case CoreEvent_Paired():
return paired(_that.name);case CoreEvent_ReceiveStarted():
return receiveStarted(_that.from,_that.files,_that.totalBytes,_that.alreadyHave);case CoreEvent_ReceiveProgress():
return receiveProgress(_that.received,_that.total);case CoreEvent_Saved():
return saved(_that.path,_that.open,_that.page);case CoreEvent_FlingUrl():
return flingUrl(_that.from,_that.url,_that.title);case CoreEvent_FlingText():
return flingText(_that.from,_that.text);case CoreEvent_ReceiveFinished():
return receiveFinished();case CoreEvent_ReceiveInterrupted():
return receiveInterrupted(_that.reason);case CoreEvent_Linked():
return linked(_that.id,_that.kind);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  devicesChanged,TResult? Function( String name)?  paired,TResult? Function( String from,  int files,  BigInt totalBytes,  BigInt alreadyHave)?  receiveStarted,TResult? Function( BigInt received,  BigInt total)?  receiveProgress,TResult? Function( String path,  bool open,  int? page)?  saved,TResult? Function( String from,  String url,  String title)?  flingUrl,TResult? Function( String from,  String text)?  flingText,TResult? Function()?  receiveFinished,TResult? Function( String reason)?  receiveInterrupted,TResult? Function( String id,  LinkedEventKind kind)?  linked,}) {final _that = this;
switch (_that) {
case CoreEvent_DevicesChanged() when devicesChanged != null:
return devicesChanged();case CoreEvent_Paired() when paired != null:
return paired(_that.name);case CoreEvent_ReceiveStarted() when receiveStarted != null:
return receiveStarted(_that.from,_that.files,_that.totalBytes,_that.alreadyHave);case CoreEvent_ReceiveProgress() when receiveProgress != null:
return receiveProgress(_that.received,_that.total);case CoreEvent_Saved() when saved != null:
return saved(_that.path,_that.open,_that.page);case CoreEvent_FlingUrl() when flingUrl != null:
return flingUrl(_that.from,_that.url,_that.title);case CoreEvent_FlingText() when flingText != null:
return flingText(_that.from,_that.text);case CoreEvent_ReceiveFinished() when receiveFinished != null:
return receiveFinished();case CoreEvent_ReceiveInterrupted() when receiveInterrupted != null:
return receiveInterrupted(_that.reason);case CoreEvent_Linked() when linked != null:
return linked(_that.id,_that.kind);case _:
  return null;

}
}

}

/// @nodoc


class CoreEvent_DevicesChanged extends CoreEvent {
  const CoreEvent_DevicesChanged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_DevicesChanged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.devicesChanged()';
}


}




/// @nodoc


class CoreEvent_Paired extends CoreEvent {
  const CoreEvent_Paired({required this.name}): super._();
  

/// Its name.
 final  String name;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_PairedCopyWith<CoreEvent_Paired> get copyWith => _$CoreEvent_PairedCopyWithImpl<CoreEvent_Paired>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_Paired&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'CoreEvent.paired(name: $name)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_PairedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_PairedCopyWith(CoreEvent_Paired value, $Res Function(CoreEvent_Paired) _then) = _$CoreEvent_PairedCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$CoreEvent_PairedCopyWithImpl<$Res>
    implements $CoreEvent_PairedCopyWith<$Res> {
  _$CoreEvent_PairedCopyWithImpl(this._self, this._then);

  final CoreEvent_Paired _self;
  final $Res Function(CoreEvent_Paired) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(CoreEvent_Paired(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreEvent_ReceiveStarted extends CoreEvent {
  const CoreEvent_ReceiveStarted({required this.from, required this.files, required this.totalBytes, required this.alreadyHave}): super._();
  

/// Short id of the sender.
 final  String from;
/// Number of files.
 final  int files;
/// Total size.
 final  BigInt totalBytes;
/// Bytes already here from an interrupted attempt.
 final  BigInt alreadyHave;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_ReceiveStartedCopyWith<CoreEvent_ReceiveStarted> get copyWith => _$CoreEvent_ReceiveStartedCopyWithImpl<CoreEvent_ReceiveStarted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ReceiveStarted&&(identical(other.from, from) || other.from == from)&&(identical(other.files, files) || other.files == files)&&(identical(other.totalBytes, totalBytes) || other.totalBytes == totalBytes)&&(identical(other.alreadyHave, alreadyHave) || other.alreadyHave == alreadyHave));
}


@override
int get hashCode {
    return Object.hash(runtimeType,from,files,totalBytes,alreadyHave);
}

@override
String toString() {
    return 'CoreEvent.receiveStarted(from: $from, files: $files, totalBytes: $totalBytes, alreadyHave: $alreadyHave)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_ReceiveStartedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_ReceiveStartedCopyWith(CoreEvent_ReceiveStarted value, $Res Function(CoreEvent_ReceiveStarted) _then) = _$CoreEvent_ReceiveStartedCopyWithImpl;
@useResult
$Res call({
 String from, int files, BigInt totalBytes, BigInt alreadyHave
});




}
/// @nodoc
class _$CoreEvent_ReceiveStartedCopyWithImpl<$Res>
    implements $CoreEvent_ReceiveStartedCopyWith<$Res> {
  _$CoreEvent_ReceiveStartedCopyWithImpl(this._self, this._then);

  final CoreEvent_ReceiveStarted _self;
  final $Res Function(CoreEvent_ReceiveStarted) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? files = null,Object? totalBytes = null,Object? alreadyHave = null,}) {
  return _then(CoreEvent_ReceiveStarted(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,files: null == files ? _self.files : files // ignore: cast_nullable_to_non_nullable
as int,totalBytes: null == totalBytes ? _self.totalBytes : totalBytes // ignore: cast_nullable_to_non_nullable
as BigInt,alreadyHave: null == alreadyHave ? _self.alreadyHave : alreadyHave // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class CoreEvent_ReceiveProgress extends CoreEvent {
  const CoreEvent_ReceiveProgress({required this.received, required this.total}): super._();
  

/// Bytes received.
 final  BigInt received;
/// Total bytes.
 final  BigInt total;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_ReceiveProgressCopyWith<CoreEvent_ReceiveProgress> get copyWith => _$CoreEvent_ReceiveProgressCopyWithImpl<CoreEvent_ReceiveProgress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ReceiveProgress&&(identical(other.received, received) || other.received == received)&&(identical(other.total, total) || other.total == total));
}


@override
int get hashCode {
    return Object.hash(runtimeType,received,total);
}

@override
String toString() {
    return 'CoreEvent.receiveProgress(received: $received, total: $total)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_ReceiveProgressCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_ReceiveProgressCopyWith(CoreEvent_ReceiveProgress value, $Res Function(CoreEvent_ReceiveProgress) _then) = _$CoreEvent_ReceiveProgressCopyWithImpl;
@useResult
$Res call({
 BigInt received, BigInt total
});




}
/// @nodoc
class _$CoreEvent_ReceiveProgressCopyWithImpl<$Res>
    implements $CoreEvent_ReceiveProgressCopyWith<$Res> {
  _$CoreEvent_ReceiveProgressCopyWithImpl(this._self, this._then);

  final CoreEvent_ReceiveProgress _self;
  final $Res Function(CoreEvent_ReceiveProgress) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? received = null,Object? total = null,}) {
  return _then(CoreEvent_ReceiveProgress(
received: null == received ? _self.received : received // ignore: cast_nullable_to_non_nullable
as BigInt,total: null == total ? _self.total : total // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class CoreEvent_Saved extends CoreEvent {
  const CoreEvent_Saved({required this.path, required this.open, this.page}): super._();
  

/// Its path.
 final  String path;
/// The sender flung it: open it now.
 final  bool open;
/// The page to open at, for documents.
 final  int? page;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_SavedCopyWith<CoreEvent_Saved> get copyWith => _$CoreEvent_SavedCopyWithImpl<CoreEvent_Saved>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_Saved&&(identical(other.path, path) || other.path == path)&&(identical(other.open, open) || other.open == open)&&(identical(other.page, page) || other.page == page));
}


@override
int get hashCode {
    return Object.hash(runtimeType,path,open,page);
}

@override
String toString() {
    return 'CoreEvent.saved(path: $path, open: $open, page: $page)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_SavedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_SavedCopyWith(CoreEvent_Saved value, $Res Function(CoreEvent_Saved) _then) = _$CoreEvent_SavedCopyWithImpl;
@useResult
$Res call({
 String path, bool open, int? page
});




}
/// @nodoc
class _$CoreEvent_SavedCopyWithImpl<$Res>
    implements $CoreEvent_SavedCopyWith<$Res> {
  _$CoreEvent_SavedCopyWithImpl(this._self, this._then);

  final CoreEvent_Saved _self;
  final $Res Function(CoreEvent_Saved) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? path = null,Object? open = null,Object? page = freezed,}) {
  return _then(CoreEvent_Saved(
path: null == path ? _self.path : path // ignore: cast_nullable_to_non_nullable
as String,open: null == open ? _self.open : open // ignore: cast_nullable_to_non_nullable
as bool,page: freezed == page ? _self.page : page // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class CoreEvent_FlingUrl extends CoreEvent {
  const CoreEvent_FlingUrl({required this.from, required this.url, required this.title}): super._();
  

/// Short id of the sender.
 final  String from;
/// The address (validated: http or https).
 final  String url;
/// The page title.
 final  String title;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_FlingUrlCopyWith<CoreEvent_FlingUrl> get copyWith => _$CoreEvent_FlingUrlCopyWithImpl<CoreEvent_FlingUrl>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_FlingUrl&&(identical(other.from, from) || other.from == from)&&(identical(other.url, url) || other.url == url)&&(identical(other.title, title) || other.title == title));
}


@override
int get hashCode {
    return Object.hash(runtimeType,from,url,title);
}

@override
String toString() {
    return 'CoreEvent.flingUrl(from: $from, url: $url, title: $title)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_FlingUrlCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_FlingUrlCopyWith(CoreEvent_FlingUrl value, $Res Function(CoreEvent_FlingUrl) _then) = _$CoreEvent_FlingUrlCopyWithImpl;
@useResult
$Res call({
 String from, String url, String title
});




}
/// @nodoc
class _$CoreEvent_FlingUrlCopyWithImpl<$Res>
    implements $CoreEvent_FlingUrlCopyWith<$Res> {
  _$CoreEvent_FlingUrlCopyWithImpl(this._self, this._then);

  final CoreEvent_FlingUrl _self;
  final $Res Function(CoreEvent_FlingUrl) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? url = null,Object? title = null,}) {
  return _then(CoreEvent_FlingUrl(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,title: null == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreEvent_FlingText extends CoreEvent {
  const CoreEvent_FlingText({required this.from, required this.text}): super._();
  

/// Short id of the sender.
 final  String from;
/// The text (validated: no control characters but line breaks and tabs).
 final  String text;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_FlingTextCopyWith<CoreEvent_FlingText> get copyWith => _$CoreEvent_FlingTextCopyWithImpl<CoreEvent_FlingText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_FlingText&&(identical(other.from, from) || other.from == from)&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,from,text);
}

@override
String toString() {
    return 'CoreEvent.flingText(from: $from, text: $text)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_FlingTextCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_FlingTextCopyWith(CoreEvent_FlingText value, $Res Function(CoreEvent_FlingText) _then) = _$CoreEvent_FlingTextCopyWithImpl;
@useResult
$Res call({
 String from, String text
});




}
/// @nodoc
class _$CoreEvent_FlingTextCopyWithImpl<$Res>
    implements $CoreEvent_FlingTextCopyWith<$Res> {
  _$CoreEvent_FlingTextCopyWithImpl(this._self, this._then);

  final CoreEvent_FlingText _self;
  final $Res Function(CoreEvent_FlingText) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? text = null,}) {
  return _then(CoreEvent_FlingText(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreEvent_ReceiveFinished extends CoreEvent {
  const CoreEvent_ReceiveFinished(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ReceiveFinished);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CoreEvent.receiveFinished()';
}


}




/// @nodoc


class CoreEvent_ReceiveInterrupted extends CoreEvent {
  const CoreEvent_ReceiveInterrupted({required this.reason}): super._();
  

/// Why.
 final  String reason;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_ReceiveInterruptedCopyWith<CoreEvent_ReceiveInterrupted> get copyWith => _$CoreEvent_ReceiveInterruptedCopyWithImpl<CoreEvent_ReceiveInterrupted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_ReceiveInterrupted&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode {
    return Object.hash(runtimeType,reason);
}

@override
String toString() {
    return 'CoreEvent.receiveInterrupted(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_ReceiveInterruptedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_ReceiveInterruptedCopyWith(CoreEvent_ReceiveInterrupted value, $Res Function(CoreEvent_ReceiveInterrupted) _then) = _$CoreEvent_ReceiveInterruptedCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$CoreEvent_ReceiveInterruptedCopyWithImpl<$Res>
    implements $CoreEvent_ReceiveInterruptedCopyWith<$Res> {
  _$CoreEvent_ReceiveInterruptedCopyWithImpl(this._self, this._then);

  final CoreEvent_ReceiveInterrupted _self;
  final $Res Function(CoreEvent_ReceiveInterrupted) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(CoreEvent_ReceiveInterrupted(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class CoreEvent_Linked extends CoreEvent {
  const CoreEvent_Linked({required this.id, required this.kind}): super._();
  

/// The link id.
 final  String id;
/// What happened.
 final  LinkedEventKind kind;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CoreEvent_LinkedCopyWith<CoreEvent_Linked> get copyWith => _$CoreEvent_LinkedCopyWithImpl<CoreEvent_Linked>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CoreEvent_Linked&&(identical(other.id, id) || other.id == id)&&(identical(other.kind, kind) || other.kind == kind));
}


@override
int get hashCode {
    return Object.hash(runtimeType,id,kind);
}

@override
String toString() {
    return 'CoreEvent.linked(id: $id, kind: $kind)';
}


}

/// @nodoc
abstract mixin class $CoreEvent_LinkedCopyWith<$Res> implements $CoreEventCopyWith<$Res> {
  factory $CoreEvent_LinkedCopyWith(CoreEvent_Linked value, $Res Function(CoreEvent_Linked) _then) = _$CoreEvent_LinkedCopyWithImpl;
@useResult
$Res call({
 String id, LinkedEventKind kind
});


$LinkedEventKindCopyWith<$Res> get kind;

}
/// @nodoc
class _$CoreEvent_LinkedCopyWithImpl<$Res>
    implements $CoreEvent_LinkedCopyWith<$Res> {
  _$CoreEvent_LinkedCopyWithImpl(this._self, this._then);

  final CoreEvent_Linked _self;
  final $Res Function(CoreEvent_Linked) _then;

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = null,Object? kind = null,}) {
  return _then(CoreEvent_Linked(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as LinkedEventKind,
  ));
}

/// Create a copy of CoreEvent
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$LinkedEventKindCopyWith<$Res> get kind {
  
  return $LinkedEventKindCopyWith<$Res>(_self.kind, (value) {
    return _then(_self.copyWith(kind: value));
  });
}
}

/// @nodoc
mixin _$LinkedEventKind {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind()';
}


}

/// @nodoc
class $LinkedEventKindCopyWith<$Res>  {
$LinkedEventKindCopyWith(LinkedEventKind _, $Res Function(LinkedEventKind) __);
}


/// Adds pattern-matching-related methods to [LinkedEventKind].
extension LinkedEventKindPatterns on LinkedEventKind {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( LinkedEventKind_Arrived value)?  arrived,TResult Function( LinkedEventKind_Updated value)?  updated,TResult Function( LinkedEventKind_Conflict value)?  conflict,TResult Function( LinkedEventKind_Resolved value)?  resolved,TResult Function( LinkedEventKind_PeerLeft value)?  peerLeft,TResult Function( LinkedEventKind_Left value)?  left,TResult Function( LinkedEventKind_Missing value)?  missing,TResult Function( LinkedEventKind_Found value)?  found,TResult Function( LinkedEventKind_Moved value)?  moved,TResult Function( LinkedEventKind_Error value)?  error,required TResult orElse(),}){
final _that = this;
switch (_that) {
case LinkedEventKind_Arrived() when arrived != null:
return arrived(_that);case LinkedEventKind_Updated() when updated != null:
return updated(_that);case LinkedEventKind_Conflict() when conflict != null:
return conflict(_that);case LinkedEventKind_Resolved() when resolved != null:
return resolved(_that);case LinkedEventKind_PeerLeft() when peerLeft != null:
return peerLeft(_that);case LinkedEventKind_Left() when left != null:
return left(_that);case LinkedEventKind_Missing() when missing != null:
return missing(_that);case LinkedEventKind_Found() when found != null:
return found(_that);case LinkedEventKind_Moved() when moved != null:
return moved(_that);case LinkedEventKind_Error() when error != null:
return error(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( LinkedEventKind_Arrived value)  arrived,required TResult Function( LinkedEventKind_Updated value)  updated,required TResult Function( LinkedEventKind_Conflict value)  conflict,required TResult Function( LinkedEventKind_Resolved value)  resolved,required TResult Function( LinkedEventKind_PeerLeft value)  peerLeft,required TResult Function( LinkedEventKind_Left value)  left,required TResult Function( LinkedEventKind_Missing value)  missing,required TResult Function( LinkedEventKind_Found value)  found,required TResult Function( LinkedEventKind_Moved value)  moved,required TResult Function( LinkedEventKind_Error value)  error,}){
final _that = this;
switch (_that) {
case LinkedEventKind_Arrived():
return arrived(_that);case LinkedEventKind_Updated():
return updated(_that);case LinkedEventKind_Conflict():
return conflict(_that);case LinkedEventKind_Resolved():
return resolved(_that);case LinkedEventKind_PeerLeft():
return peerLeft(_that);case LinkedEventKind_Left():
return left(_that);case LinkedEventKind_Missing():
return missing(_that);case LinkedEventKind_Found():
return found(_that);case LinkedEventKind_Moved():
return moved(_that);case LinkedEventKind_Error():
return error(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( LinkedEventKind_Arrived value)?  arrived,TResult? Function( LinkedEventKind_Updated value)?  updated,TResult? Function( LinkedEventKind_Conflict value)?  conflict,TResult? Function( LinkedEventKind_Resolved value)?  resolved,TResult? Function( LinkedEventKind_PeerLeft value)?  peerLeft,TResult? Function( LinkedEventKind_Left value)?  left,TResult? Function( LinkedEventKind_Missing value)?  missing,TResult? Function( LinkedEventKind_Found value)?  found,TResult? Function( LinkedEventKind_Moved value)?  moved,TResult? Function( LinkedEventKind_Error value)?  error,}){
final _that = this;
switch (_that) {
case LinkedEventKind_Arrived() when arrived != null:
return arrived(_that);case LinkedEventKind_Updated() when updated != null:
return updated(_that);case LinkedEventKind_Conflict() when conflict != null:
return conflict(_that);case LinkedEventKind_Resolved() when resolved != null:
return resolved(_that);case LinkedEventKind_PeerLeft() when peerLeft != null:
return peerLeft(_that);case LinkedEventKind_Left() when left != null:
return left(_that);case LinkedEventKind_Missing() when missing != null:
return missing(_that);case LinkedEventKind_Found() when found != null:
return found(_that);case LinkedEventKind_Moved() when moved != null:
return moved(_that);case LinkedEventKind_Error() when error != null:
return error(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String path,  String from)?  arrived,TResult Function()?  updated,TResult Function()?  conflict,TResult Function()?  resolved,TResult Function( String device)?  peerLeft,TResult Function()?  left,TResult Function()?  missing,TResult Function()?  found,TResult Function()?  moved,TResult Function( String message)?  error,required TResult orElse(),}) {final _that = this;
switch (_that) {
case LinkedEventKind_Arrived() when arrived != null:
return arrived(_that.path,_that.from);case LinkedEventKind_Updated() when updated != null:
return updated();case LinkedEventKind_Conflict() when conflict != null:
return conflict();case LinkedEventKind_Resolved() when resolved != null:
return resolved();case LinkedEventKind_PeerLeft() when peerLeft != null:
return peerLeft(_that.device);case LinkedEventKind_Left() when left != null:
return left();case LinkedEventKind_Missing() when missing != null:
return missing();case LinkedEventKind_Found() when found != null:
return found();case LinkedEventKind_Moved() when moved != null:
return moved();case LinkedEventKind_Error() when error != null:
return error(_that.message);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String path,  String from)  arrived,required TResult Function()  updated,required TResult Function()  conflict,required TResult Function()  resolved,required TResult Function( String device)  peerLeft,required TResult Function()  left,required TResult Function()  missing,required TResult Function()  found,required TResult Function()  moved,required TResult Function( String message)  error,}) {final _that = this;
switch (_that) {
case LinkedEventKind_Arrived():
return arrived(_that.path,_that.from);case LinkedEventKind_Updated():
return updated();case LinkedEventKind_Conflict():
return conflict();case LinkedEventKind_Resolved():
return resolved();case LinkedEventKind_PeerLeft():
return peerLeft(_that.device);case LinkedEventKind_Left():
return left();case LinkedEventKind_Missing():
return missing();case LinkedEventKind_Found():
return found();case LinkedEventKind_Moved():
return moved();case LinkedEventKind_Error():
return error(_that.message);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String path,  String from)?  arrived,TResult? Function()?  updated,TResult? Function()?  conflict,TResult? Function()?  resolved,TResult? Function( String device)?  peerLeft,TResult? Function()?  left,TResult? Function()?  missing,TResult? Function()?  found,TResult? Function()?  moved,TResult? Function( String message)?  error,}) {final _that = this;
switch (_that) {
case LinkedEventKind_Arrived() when arrived != null:
return arrived(_that.path,_that.from);case LinkedEventKind_Updated() when updated != null:
return updated();case LinkedEventKind_Conflict() when conflict != null:
return conflict();case LinkedEventKind_Resolved() when resolved != null:
return resolved();case LinkedEventKind_PeerLeft() when peerLeft != null:
return peerLeft(_that.device);case LinkedEventKind_Left() when left != null:
return left();case LinkedEventKind_Missing() when missing != null:
return missing();case LinkedEventKind_Found() when found != null:
return found();case LinkedEventKind_Moved() when moved != null:
return moved();case LinkedEventKind_Error() when error != null:
return error(_that.message);case _:
  return null;

}
}

}

/// @nodoc


class LinkedEventKind_Arrived extends LinkedEventKind {
  const LinkedEventKind_Arrived({required this.path, required this.from}): super._();
  

/// Where it was saved.
 final  String path;
/// The sending device's name.
 final  String from;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LinkedEventKind_ArrivedCopyWith<LinkedEventKind_Arrived> get copyWith => _$LinkedEventKind_ArrivedCopyWithImpl<LinkedEventKind_Arrived>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Arrived&&(identical(other.path, path) || other.path == path)&&(identical(other.from, from) || other.from == from));
}


@override
int get hashCode {
    return Object.hash(runtimeType,path,from);
}

@override
String toString() {
    return 'LinkedEventKind.arrived(path: $path, from: $from)';
}


}

/// @nodoc
abstract mixin class $LinkedEventKind_ArrivedCopyWith<$Res> implements $LinkedEventKindCopyWith<$Res> {
  factory $LinkedEventKind_ArrivedCopyWith(LinkedEventKind_Arrived value, $Res Function(LinkedEventKind_Arrived) _then) = _$LinkedEventKind_ArrivedCopyWithImpl;
@useResult
$Res call({
 String path, String from
});




}
/// @nodoc
class _$LinkedEventKind_ArrivedCopyWithImpl<$Res>
    implements $LinkedEventKind_ArrivedCopyWith<$Res> {
  _$LinkedEventKind_ArrivedCopyWithImpl(this._self, this._then);

  final LinkedEventKind_Arrived _self;
  final $Res Function(LinkedEventKind_Arrived) _then;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? path = null,Object? from = null,}) {
  return _then(LinkedEventKind_Arrived(
path: null == path ? _self.path : path // ignore: cast_nullable_to_non_nullable
as String,from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LinkedEventKind_Updated extends LinkedEventKind {
  const LinkedEventKind_Updated(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Updated);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.updated()';
}


}




/// @nodoc


class LinkedEventKind_Conflict extends LinkedEventKind {
  const LinkedEventKind_Conflict(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Conflict);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.conflict()';
}


}




/// @nodoc


class LinkedEventKind_Resolved extends LinkedEventKind {
  const LinkedEventKind_Resolved(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Resolved);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.resolved()';
}


}




/// @nodoc


class LinkedEventKind_PeerLeft extends LinkedEventKind {
  const LinkedEventKind_PeerLeft({required this.device}): super._();
  

/// Its name.
 final  String device;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LinkedEventKind_PeerLeftCopyWith<LinkedEventKind_PeerLeft> get copyWith => _$LinkedEventKind_PeerLeftCopyWithImpl<LinkedEventKind_PeerLeft>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_PeerLeft&&(identical(other.device, device) || other.device == device));
}


@override
int get hashCode {
    return Object.hash(runtimeType,device);
}

@override
String toString() {
    return 'LinkedEventKind.peerLeft(device: $device)';
}


}

/// @nodoc
abstract mixin class $LinkedEventKind_PeerLeftCopyWith<$Res> implements $LinkedEventKindCopyWith<$Res> {
  factory $LinkedEventKind_PeerLeftCopyWith(LinkedEventKind_PeerLeft value, $Res Function(LinkedEventKind_PeerLeft) _then) = _$LinkedEventKind_PeerLeftCopyWithImpl;
@useResult
$Res call({
 String device
});




}
/// @nodoc
class _$LinkedEventKind_PeerLeftCopyWithImpl<$Res>
    implements $LinkedEventKind_PeerLeftCopyWith<$Res> {
  _$LinkedEventKind_PeerLeftCopyWithImpl(this._self, this._then);

  final LinkedEventKind_PeerLeft _self;
  final $Res Function(LinkedEventKind_PeerLeft) _then;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? device = null,}) {
  return _then(LinkedEventKind_PeerLeft(
device: null == device ? _self.device : device // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LinkedEventKind_Left extends LinkedEventKind {
  const LinkedEventKind_Left(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Left);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.left()';
}


}




/// @nodoc


class LinkedEventKind_Missing extends LinkedEventKind {
  const LinkedEventKind_Missing(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Missing);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.missing()';
}


}




/// @nodoc


class LinkedEventKind_Found extends LinkedEventKind {
  const LinkedEventKind_Found(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Found);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.found()';
}


}




/// @nodoc


class LinkedEventKind_Moved extends LinkedEventKind {
  const LinkedEventKind_Moved(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Moved);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkedEventKind.moved()';
}


}




/// @nodoc


class LinkedEventKind_Error extends LinkedEventKind {
  const LinkedEventKind_Error({required this.message}): super._();
  

/// What.
 final  String message;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LinkedEventKind_ErrorCopyWith<LinkedEventKind_Error> get copyWith => _$LinkedEventKind_ErrorCopyWithImpl<LinkedEventKind_Error>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkedEventKind_Error&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,message);
}

@override
String toString() {
    return 'LinkedEventKind.error(message: $message)';
}


}

/// @nodoc
abstract mixin class $LinkedEventKind_ErrorCopyWith<$Res> implements $LinkedEventKindCopyWith<$Res> {
  factory $LinkedEventKind_ErrorCopyWith(LinkedEventKind_Error value, $Res Function(LinkedEventKind_Error) _then) = _$LinkedEventKind_ErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LinkedEventKind_ErrorCopyWithImpl<$Res>
    implements $LinkedEventKind_ErrorCopyWith<$Res> {
  _$LinkedEventKind_ErrorCopyWithImpl(this._self, this._then);

  final LinkedEventKind_Error _self;
  final $Res Function(LinkedEventKind_Error) _then;

/// Create a copy of LinkedEventKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LinkedEventKind_Error(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$PairingEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PairingEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'PairingEvent()';
}


}

/// @nodoc
class $PairingEventCopyWith<$Res>  {
$PairingEventCopyWith(PairingEvent _, $Res Function(PairingEvent) __);
}


/// Adds pattern-matching-related methods to [PairingEvent].
extension PairingEventPatterns on PairingEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PairingEvent_Invite value)?  invite,TResult Function( PairingEvent_Confirm value)?  confirm,TResult Function( PairingEvent_Paired value)?  paired,TResult Function( PairingEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PairingEvent_Invite() when invite != null:
return invite(_that);case PairingEvent_Confirm() when confirm != null:
return confirm(_that);case PairingEvent_Paired() when paired != null:
return paired(_that);case PairingEvent_Failed() when failed != null:
return failed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PairingEvent_Invite value)  invite,required TResult Function( PairingEvent_Confirm value)  confirm,required TResult Function( PairingEvent_Paired value)  paired,required TResult Function( PairingEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case PairingEvent_Invite():
return invite(_that);case PairingEvent_Confirm():
return confirm(_that);case PairingEvent_Paired():
return paired(_that);case PairingEvent_Failed():
return failed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PairingEvent_Invite value)?  invite,TResult? Function( PairingEvent_Confirm value)?  confirm,TResult? Function( PairingEvent_Paired value)?  paired,TResult? Function( PairingEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case PairingEvent_Invite() when invite != null:
return invite(_that);case PairingEvent_Confirm() when confirm != null:
return confirm(_that);case PairingEvent_Paired() when paired != null:
return paired(_that);case PairingEvent_Failed() when failed != null:
return failed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String uri)?  invite,TResult Function( String peerName,  String shortId,  String code)?  confirm,TResult Function( String name)?  paired,TResult Function( String reason)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PairingEvent_Invite() when invite != null:
return invite(_that.uri);case PairingEvent_Confirm() when confirm != null:
return confirm(_that.peerName,_that.shortId,_that.code);case PairingEvent_Paired() when paired != null:
return paired(_that.name);case PairingEvent_Failed() when failed != null:
return failed(_that.reason);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String uri)  invite,required TResult Function( String peerName,  String shortId,  String code)  confirm,required TResult Function( String name)  paired,required TResult Function( String reason)  failed,}) {final _that = this;
switch (_that) {
case PairingEvent_Invite():
return invite(_that.uri);case PairingEvent_Confirm():
return confirm(_that.peerName,_that.shortId,_that.code);case PairingEvent_Paired():
return paired(_that.name);case PairingEvent_Failed():
return failed(_that.reason);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String uri)?  invite,TResult? Function( String peerName,  String shortId,  String code)?  confirm,TResult? Function( String name)?  paired,TResult? Function( String reason)?  failed,}) {final _that = this;
switch (_that) {
case PairingEvent_Invite() when invite != null:
return invite(_that.uri);case PairingEvent_Confirm() when confirm != null:
return confirm(_that.peerName,_that.shortId,_that.code);case PairingEvent_Paired() when paired != null:
return paired(_that.name);case PairingEvent_Failed() when failed != null:
return failed(_that.reason);case _:
  return null;

}
}

}

/// @nodoc


class PairingEvent_Invite extends PairingEvent {
  const PairingEvent_Invite({required this.uri}): super._();
  

/// The `ecpair:` link.
 final  String uri;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PairingEvent_InviteCopyWith<PairingEvent_Invite> get copyWith => _$PairingEvent_InviteCopyWithImpl<PairingEvent_Invite>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PairingEvent_Invite&&(identical(other.uri, uri) || other.uri == uri));
}


@override
int get hashCode {
    return Object.hash(runtimeType,uri);
}

@override
String toString() {
    return 'PairingEvent.invite(uri: $uri)';
}


}

/// @nodoc
abstract mixin class $PairingEvent_InviteCopyWith<$Res> implements $PairingEventCopyWith<$Res> {
  factory $PairingEvent_InviteCopyWith(PairingEvent_Invite value, $Res Function(PairingEvent_Invite) _then) = _$PairingEvent_InviteCopyWithImpl;
@useResult
$Res call({
 String uri
});




}
/// @nodoc
class _$PairingEvent_InviteCopyWithImpl<$Res>
    implements $PairingEvent_InviteCopyWith<$Res> {
  _$PairingEvent_InviteCopyWithImpl(this._self, this._then);

  final PairingEvent_Invite _self;
  final $Res Function(PairingEvent_Invite) _then;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? uri = null,}) {
  return _then(PairingEvent_Invite(
uri: null == uri ? _self.uri : uri // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PairingEvent_Confirm extends PairingEvent {
  const PairingEvent_Confirm({required this.peerName, required this.shortId, required this.code}): super._();
  

/// The other device's name.
 final  String peerName;
/// Its short id.
 final  String shortId;
/// The six-digit code, as `"042 917"`.
 final  String code;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PairingEvent_ConfirmCopyWith<PairingEvent_Confirm> get copyWith => _$PairingEvent_ConfirmCopyWithImpl<PairingEvent_Confirm>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PairingEvent_Confirm&&(identical(other.peerName, peerName) || other.peerName == peerName)&&(identical(other.shortId, shortId) || other.shortId == shortId)&&(identical(other.code, code) || other.code == code));
}


@override
int get hashCode {
    return Object.hash(runtimeType,peerName,shortId,code);
}

@override
String toString() {
    return 'PairingEvent.confirm(peerName: $peerName, shortId: $shortId, code: $code)';
}


}

/// @nodoc
abstract mixin class $PairingEvent_ConfirmCopyWith<$Res> implements $PairingEventCopyWith<$Res> {
  factory $PairingEvent_ConfirmCopyWith(PairingEvent_Confirm value, $Res Function(PairingEvent_Confirm) _then) = _$PairingEvent_ConfirmCopyWithImpl;
@useResult
$Res call({
 String peerName, String shortId, String code
});




}
/// @nodoc
class _$PairingEvent_ConfirmCopyWithImpl<$Res>
    implements $PairingEvent_ConfirmCopyWith<$Res> {
  _$PairingEvent_ConfirmCopyWithImpl(this._self, this._then);

  final PairingEvent_Confirm _self;
  final $Res Function(PairingEvent_Confirm) _then;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? peerName = null,Object? shortId = null,Object? code = null,}) {
  return _then(PairingEvent_Confirm(
peerName: null == peerName ? _self.peerName : peerName // ignore: cast_nullable_to_non_nullable
as String,shortId: null == shortId ? _self.shortId : shortId // ignore: cast_nullable_to_non_nullable
as String,code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PairingEvent_Paired extends PairingEvent {
  const PairingEvent_Paired({required this.name}): super._();
  

/// The other device's name.
 final  String name;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PairingEvent_PairedCopyWith<PairingEvent_Paired> get copyWith => _$PairingEvent_PairedCopyWithImpl<PairingEvent_Paired>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PairingEvent_Paired&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'PairingEvent.paired(name: $name)';
}


}

/// @nodoc
abstract mixin class $PairingEvent_PairedCopyWith<$Res> implements $PairingEventCopyWith<$Res> {
  factory $PairingEvent_PairedCopyWith(PairingEvent_Paired value, $Res Function(PairingEvent_Paired) _then) = _$PairingEvent_PairedCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$PairingEvent_PairedCopyWithImpl<$Res>
    implements $PairingEvent_PairedCopyWith<$Res> {
  _$PairingEvent_PairedCopyWithImpl(this._self, this._then);

  final PairingEvent_Paired _self;
  final $Res Function(PairingEvent_Paired) _then;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(PairingEvent_Paired(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PairingEvent_Failed extends PairingEvent {
  const PairingEvent_Failed({required this.reason}): super._();
  

/// Why.
 final  String reason;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PairingEvent_FailedCopyWith<PairingEvent_Failed> get copyWith => _$PairingEvent_FailedCopyWithImpl<PairingEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PairingEvent_Failed&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode {
    return Object.hash(runtimeType,reason);
}

@override
String toString() {
    return 'PairingEvent.failed(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $PairingEvent_FailedCopyWith<$Res> implements $PairingEventCopyWith<$Res> {
  factory $PairingEvent_FailedCopyWith(PairingEvent_Failed value, $Res Function(PairingEvent_Failed) _then) = _$PairingEvent_FailedCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$PairingEvent_FailedCopyWithImpl<$Res>
    implements $PairingEvent_FailedCopyWith<$Res> {
  _$PairingEvent_FailedCopyWithImpl(this._self, this._then);

  final PairingEvent_Failed _self;
  final $Res Function(PairingEvent_Failed) _then;

/// Create a copy of PairingEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(PairingEvent_Failed(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$SendUpdate {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SendUpdate);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'SendUpdate()';
}


}

/// @nodoc
class $SendUpdateCopyWith<$Res>  {
$SendUpdateCopyWith(SendUpdate _, $Res Function(SendUpdate) __);
}


/// Adds pattern-matching-related methods to [SendUpdate].
extension SendUpdatePatterns on SendUpdate {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SendUpdate_Progress value)?  progress,TResult Function( SendUpdate_Retrying value)?  retrying,TResult Function( SendUpdate_Done value)?  done,TResult Function( SendUpdate_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SendUpdate_Progress() when progress != null:
return progress(_that);case SendUpdate_Retrying() when retrying != null:
return retrying(_that);case SendUpdate_Done() when done != null:
return done(_that);case SendUpdate_Failed() when failed != null:
return failed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SendUpdate_Progress value)  progress,required TResult Function( SendUpdate_Retrying value)  retrying,required TResult Function( SendUpdate_Done value)  done,required TResult Function( SendUpdate_Failed value)  failed,}){
final _that = this;
switch (_that) {
case SendUpdate_Progress():
return progress(_that);case SendUpdate_Retrying():
return retrying(_that);case SendUpdate_Done():
return done(_that);case SendUpdate_Failed():
return failed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SendUpdate_Progress value)?  progress,TResult? Function( SendUpdate_Retrying value)?  retrying,TResult? Function( SendUpdate_Done value)?  done,TResult? Function( SendUpdate_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case SendUpdate_Progress() when progress != null:
return progress(_that);case SendUpdate_Retrying() when retrying != null:
return retrying(_that);case SendUpdate_Done() when done != null:
return done(_that);case SendUpdate_Failed() when failed != null:
return failed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( BigInt sent,  BigInt toSend,  int paths)?  progress,TResult Function( String reason)?  retrying,TResult Function( int files,  BigInt bytes,  BigInt millis,  int paths)?  done,TResult Function( String reason)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SendUpdate_Progress() when progress != null:
return progress(_that.sent,_that.toSend,_that.paths);case SendUpdate_Retrying() when retrying != null:
return retrying(_that.reason);case SendUpdate_Done() when done != null:
return done(_that.files,_that.bytes,_that.millis,_that.paths);case SendUpdate_Failed() when failed != null:
return failed(_that.reason);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( BigInt sent,  BigInt toSend,  int paths)  progress,required TResult Function( String reason)  retrying,required TResult Function( int files,  BigInt bytes,  BigInt millis,  int paths)  done,required TResult Function( String reason)  failed,}) {final _that = this;
switch (_that) {
case SendUpdate_Progress():
return progress(_that.sent,_that.toSend,_that.paths);case SendUpdate_Retrying():
return retrying(_that.reason);case SendUpdate_Done():
return done(_that.files,_that.bytes,_that.millis,_that.paths);case SendUpdate_Failed():
return failed(_that.reason);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( BigInt sent,  BigInt toSend,  int paths)?  progress,TResult? Function( String reason)?  retrying,TResult? Function( int files,  BigInt bytes,  BigInt millis,  int paths)?  done,TResult? Function( String reason)?  failed,}) {final _that = this;
switch (_that) {
case SendUpdate_Progress() when progress != null:
return progress(_that.sent,_that.toSend,_that.paths);case SendUpdate_Retrying() when retrying != null:
return retrying(_that.reason);case SendUpdate_Done() when done != null:
return done(_that.files,_that.bytes,_that.millis,_that.paths);case SendUpdate_Failed() when failed != null:
return failed(_that.reason);case _:
  return null;

}
}

}

/// @nodoc


class SendUpdate_Progress extends SendUpdate {
  const SendUpdate_Progress({required this.sent, required this.toSend, required this.paths}): super._();
  

/// Sent so far.
 final  BigInt sent;
/// To send in this attempt.
 final  BigInt toSend;
/// Network paths in use.
 final  int paths;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SendUpdate_ProgressCopyWith<SendUpdate_Progress> get copyWith => _$SendUpdate_ProgressCopyWithImpl<SendUpdate_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SendUpdate_Progress&&(identical(other.sent, sent) || other.sent == sent)&&(identical(other.toSend, toSend) || other.toSend == toSend)&&(identical(other.paths, paths) || other.paths == paths));
}


@override
int get hashCode {
    return Object.hash(runtimeType,sent,toSend,paths);
}

@override
String toString() {
    return 'SendUpdate.progress(sent: $sent, toSend: $toSend, paths: $paths)';
}


}

/// @nodoc
abstract mixin class $SendUpdate_ProgressCopyWith<$Res> implements $SendUpdateCopyWith<$Res> {
  factory $SendUpdate_ProgressCopyWith(SendUpdate_Progress value, $Res Function(SendUpdate_Progress) _then) = _$SendUpdate_ProgressCopyWithImpl;
@useResult
$Res call({
 BigInt sent, BigInt toSend, int paths
});




}
/// @nodoc
class _$SendUpdate_ProgressCopyWithImpl<$Res>
    implements $SendUpdate_ProgressCopyWith<$Res> {
  _$SendUpdate_ProgressCopyWithImpl(this._self, this._then);

  final SendUpdate_Progress _self;
  final $Res Function(SendUpdate_Progress) _then;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? sent = null,Object? toSend = null,Object? paths = null,}) {
  return _then(SendUpdate_Progress(
sent: null == sent ? _self.sent : sent // ignore: cast_nullable_to_non_nullable
as BigInt,toSend: null == toSend ? _self.toSend : toSend // ignore: cast_nullable_to_non_nullable
as BigInt,paths: null == paths ? _self.paths : paths // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SendUpdate_Retrying extends SendUpdate {
  const SendUpdate_Retrying({required this.reason}): super._();
  

/// Why.
 final  String reason;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SendUpdate_RetryingCopyWith<SendUpdate_Retrying> get copyWith => _$SendUpdate_RetryingCopyWithImpl<SendUpdate_Retrying>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SendUpdate_Retrying&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode {
    return Object.hash(runtimeType,reason);
}

@override
String toString() {
    return 'SendUpdate.retrying(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SendUpdate_RetryingCopyWith<$Res> implements $SendUpdateCopyWith<$Res> {
  factory $SendUpdate_RetryingCopyWith(SendUpdate_Retrying value, $Res Function(SendUpdate_Retrying) _then) = _$SendUpdate_RetryingCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$SendUpdate_RetryingCopyWithImpl<$Res>
    implements $SendUpdate_RetryingCopyWith<$Res> {
  _$SendUpdate_RetryingCopyWithImpl(this._self, this._then);

  final SendUpdate_Retrying _self;
  final $Res Function(SendUpdate_Retrying) _then;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SendUpdate_Retrying(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SendUpdate_Done extends SendUpdate {
  const SendUpdate_Done({required this.files, required this.bytes, required this.millis, required this.paths}): super._();
  

/// Number of files.
 final  int files;
/// Bytes sent.
 final  BigInt bytes;
/// Duration in milliseconds.
 final  BigInt millis;
/// Paths used.
 final  int paths;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SendUpdate_DoneCopyWith<SendUpdate_Done> get copyWith => _$SendUpdate_DoneCopyWithImpl<SendUpdate_Done>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SendUpdate_Done&&(identical(other.files, files) || other.files == files)&&(identical(other.bytes, bytes) || other.bytes == bytes)&&(identical(other.millis, millis) || other.millis == millis)&&(identical(other.paths, paths) || other.paths == paths));
}


@override
int get hashCode {
    return Object.hash(runtimeType,files,bytes,millis,paths);
}

@override
String toString() {
    return 'SendUpdate.done(files: $files, bytes: $bytes, millis: $millis, paths: $paths)';
}


}

/// @nodoc
abstract mixin class $SendUpdate_DoneCopyWith<$Res> implements $SendUpdateCopyWith<$Res> {
  factory $SendUpdate_DoneCopyWith(SendUpdate_Done value, $Res Function(SendUpdate_Done) _then) = _$SendUpdate_DoneCopyWithImpl;
@useResult
$Res call({
 int files, BigInt bytes, BigInt millis, int paths
});




}
/// @nodoc
class _$SendUpdate_DoneCopyWithImpl<$Res>
    implements $SendUpdate_DoneCopyWith<$Res> {
  _$SendUpdate_DoneCopyWithImpl(this._self, this._then);

  final SendUpdate_Done _self;
  final $Res Function(SendUpdate_Done) _then;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? files = null,Object? bytes = null,Object? millis = null,Object? paths = null,}) {
  return _then(SendUpdate_Done(
files: null == files ? _self.files : files // ignore: cast_nullable_to_non_nullable
as int,bytes: null == bytes ? _self.bytes : bytes // ignore: cast_nullable_to_non_nullable
as BigInt,millis: null == millis ? _self.millis : millis // ignore: cast_nullable_to_non_nullable
as BigInt,paths: null == paths ? _self.paths : paths // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SendUpdate_Failed extends SendUpdate {
  const SendUpdate_Failed({required this.reason}): super._();
  

/// Why.
 final  String reason;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SendUpdate_FailedCopyWith<SendUpdate_Failed> get copyWith => _$SendUpdate_FailedCopyWithImpl<SendUpdate_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SendUpdate_Failed&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode {
    return Object.hash(runtimeType,reason);
}

@override
String toString() {
    return 'SendUpdate.failed(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SendUpdate_FailedCopyWith<$Res> implements $SendUpdateCopyWith<$Res> {
  factory $SendUpdate_FailedCopyWith(SendUpdate_Failed value, $Res Function(SendUpdate_Failed) _then) = _$SendUpdate_FailedCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$SendUpdate_FailedCopyWithImpl<$Res>
    implements $SendUpdate_FailedCopyWith<$Res> {
  _$SendUpdate_FailedCopyWithImpl(this._self, this._then);

  final SendUpdate_Failed _self;
  final $Res Function(SendUpdate_Failed) _then;

/// Create a copy of SendUpdate
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SendUpdate_Failed(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
