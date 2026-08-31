fn void test_math() @test { // this very long comment exists to check the f...
	int x = 10; // this very long comment exists to actually check the bug with the view not following logpos
	int y = 20;
	const int xy1020 = 0xFF + 0b0101 - 0o77u + 9ull;
	test::eq(x + y, 40);
	// Test failed ^^^ ( example.c3:4 ) `30` != `40`
}