import 'package:ec_app/ec_app.dart';
import 'package:ec_windows/arrivals.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a page shows its host and offers to open it in the browser', () {
    final entry = windowsArrival(
      const ArrivalInfo.page(
        'https://example.com/a/b',
        title: 'A page',
        from: 'Phone',
      ),
    );
    expect(entry.data.isPage, isTrue);
    expect(entry.data.title, 'A page');
    expect(entry.data.detail, 'example.com');
    expect(entry.onOpen, isNull);
  });

  test('text shows its first line as the title and the rest as detail', () {
    final entry = windowsArrival(
      const ArrivalInfo.text('First line\nsecond\nthird', from: 'Phone'),
    );
    expect(entry.data.isText, isTrue);
    expect(entry.data.title, 'First line');
    expect(entry.data.detail, 'second third');
    expect(entry.onOpen, isNull);
    expect(entry.onSaveElsewhere, isNull);
  });

  test('one file can be opened, saved elsewhere and shown as an image', () {
    final entry = windowsArrival(
      ArrivalInfo.files(
        const [r'C:\Users\me\Downloads\ECOnnector\photo.JPG'],
        from: 'Phone',
        size: 2048,
      ),
    );
    expect(entry.data.title, 'photo.JPG');
    expect(entry.data.imagePath, r'C:\Users\me\Downloads\ECOnnector\photo.JPG');
    expect(entry.data.canSaveElsewhere, isTrue);
    expect(entry.onOpen, isNotNull);
    expect(entry.onSaveElsewhere, isNotNull);
  });

  test('several files offer no open or save, and no preview', () {
    final entry = windowsArrival(
      ArrivalInfo.files(const ['a.txt', 'b.txt'], from: 'Phone', size: 10),
    );
    expect(entry.data.title, '2 files');
    expect(entry.data.imagePath, isNull);
    expect(entry.data.canSaveElsewhere, isFalse);
    expect(entry.onOpen, isNull);
  });
}
